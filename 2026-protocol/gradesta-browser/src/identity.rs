use anyhow::{anyhow, Context, Result};
use p256::ecdsa::{signature::Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Stored identity for a Nextcloud account
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub display_name: String,     // user@server
    pub nextcloud_url: String,    // https://nextcloud.example.com
    pub username: String,
    pub app_password: String,
    pub share_url: String,        // Public share URL for identity.pub
    #[serde(default)]
    pub remembered_servers: Vec<String>, // Server URLs that user has chosen to remember
    #[serde(skip)]
    pub signing_key: Option<SigningKey>, // Loaded at runtime
}

/// Configuration file structure
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub identities: Vec<Identity>,
}

impl IdentityConfig {
    /// Load config from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(&path).context("Failed to read config")?;
        let config: Self = serde_json::from_str(&data).context("Failed to parse config")?;
        Ok(config)
    }

    /// Save config to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config dir")?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&path, data).context("Failed to write config")?;
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().ok_or_else(|| anyhow!("No config dir"))?;
        Ok(config_dir.join("gradesta").join("identities.json"))
    }
}

/// Nextcloud Login Flow v2 response from initial POST
#[derive(Debug, Deserialize)]
struct LoginFlowInit {
    poll: LoginFlowPoll,
    login: String,
}

#[derive(Debug, Deserialize)]
struct LoginFlowPoll {
    token: String,
    endpoint: String,
}

/// Nextcloud Login Flow v2 poll response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginFlowResult {
    server: String,
    login_name: String,
    app_password: String,
}

/// OCS response structure for share info
#[derive(Debug, Deserialize)]
struct OcsData<T> {
    data: T,
}

/// Initiates Nextcloud Login Flow v2 and returns the URL to open in browser
pub fn initiate_nextcloud_login(nextcloud_url: &str) -> Result<(String, String, String)> {
    let client = reqwest::blocking::Client::new();
    let login_endpoint = format!("{}/index.php/login/v2", nextcloud_url.trim_end_matches('/'));

    let resp: LoginFlowInit = client
        .post(&login_endpoint)
        .send()
        .context("Failed to initiate login flow")?
        .json()
        .context("Failed to parse login flow response")?;

    // Return: (login_url, poll_endpoint, poll_token)
    Ok((resp.login, resp.poll.endpoint, resp.poll.token))
}

/// Polls for login completion
pub fn poll_login_completion(poll_endpoint: &str, poll_token: &str) -> Result<Option<(String, String, String)>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let resp = client
        .post(poll_endpoint)
        .form(&[("token", poll_token)])
        .send();

    match resp {
        Ok(r) if r.status().is_success() => {
            let result: LoginFlowResult = r.json().context("Failed to parse login result")?;
            // Return: (server, username, app_password)
            Ok(Some((result.server, result.login_name, result.app_password)))
        }
        Ok(r) if r.status().as_u16() == 404 => {
            // Not yet completed
            Ok(None)
        }
        Ok(r) => Err(anyhow!("Unexpected status: {}", r.status())),
        Err(e) if e.is_timeout() => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Generate a new ECDSA P-256 keypair
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = *signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Serialize public key to uncompressed point format (65 bytes: 0x04 || X || Y)
pub fn serialize_public_key(key: &VerifyingKey) -> Vec<u8> {
    use p256::EncodedPoint;
    let point: EncodedPoint = key.to_encoded_point(false);
    point.as_bytes().to_vec()
}

/// Upload a file to Nextcloud via WebDAV
pub fn webdav_upload(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
    path: &str,
    content: &[u8],
) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}/remote.php/dav/files/{}/{}",
        nextcloud_url.trim_end_matches('/'),
        username,
        path.trim_start_matches('/')
    );

    // First, try to create the parent directory
    let parent_path = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
    if !parent_path.is_empty() {
        let dir_url = format!(
            "{}/remote.php/dav/files/{}/{}",
            nextcloud_url.trim_end_matches('/'),
            username,
            parent_path.trim_start_matches('/')
        );
        // MKCOL to create directory (ignore errors if it already exists)
        let _ = client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &dir_url)
            .basic_auth(username, Some(app_password))
            .send();
    }

    // Upload the file
    let resp = client
        .put(&url)
        .basic_auth(username, Some(app_password))
        .body(content.to_vec())
        .send()
        .context("WebDAV PUT failed")?;

    if !resp.status().is_success() && resp.status().as_u16() != 201 && resp.status().as_u16() != 204 {
        return Err(anyhow!("WebDAV upload failed: {}", resp.status()));
    }

    Ok(())
}

/// Download a file from Nextcloud via WebDAV
pub fn webdav_download(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
    path: &str,
) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}/remote.php/dav/files/{}/{}",
        nextcloud_url.trim_end_matches('/'),
        username,
        path.trim_start_matches('/')
    );

    let resp = client
        .get(&url)
        .basic_auth(username, Some(app_password))
        .send()
        .context("WebDAV GET failed")?;

    if !resp.status().is_success() {
        return Err(anyhow!("WebDAV download failed: {}", resp.status()));
    }

    Ok(resp.bytes()?.to_vec())
}

/// Create a public share for a file and return the share URL
pub fn create_public_share(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
    path: &str,
) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}/ocs/v2.php/apps/files_sharing/api/v1/shares",
        nextcloud_url.trim_end_matches('/')
    );

    #[derive(Deserialize)]
    struct ShareResponse {
        ocs: OcsData<ShareData>,
    }

    #[derive(Deserialize)]
    struct ShareData {
        url: String,
    }

    let resp = client
        .post(&url)
        .basic_auth(username, Some(app_password))
        .header("OCS-APIRequest", "true")
        .form(&[
            ("path", path),
            ("shareType", "3"), // 3 = public link
            ("permissions", "1"), // 1 = read
        ])
        .send()
        .context("Failed to create share")?;

    if !resp.status().is_success() {
        return Err(anyhow!("Share creation failed: {}", resp.status()));
    }

    let share: ShareResponse = resp.json().context("Failed to parse share response")?;
    Ok(share.ocs.data.url)
}

/// Sign a challenge (nonce || timestamp) with the signing key
pub fn sign_challenge(signing_key: &SigningKey, nonce: &[u8], timestamp: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(nonce.len() + 8);
    data.extend_from_slice(nonce);
    data.extend_from_slice(&timestamp.to_be_bytes());

    // Hash and sign
    let hash = Sha256::digest(&data);
    let signature: p256::ecdsa::Signature = signing_key.sign(&hash);
    signature.to_bytes().to_vec()
}

/// Complete identity setup: generate keys, upload to Nextcloud, create share
pub fn setup_identity(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
) -> Result<(SigningKey, String)> {
    // Generate keypair
    let (signing_key, verifying_key) = generate_keypair();

    // Serialize public key
    let pub_key_bytes = serialize_public_key(&verifying_key);

    // Upload public key
    webdav_upload(
        nextcloud_url,
        username,
        app_password,
        ".gradesta/identity.pub",
        &pub_key_bytes,
    )?;

    // For the private key, we store it encrypted
    // For simplicity, we'll just store the raw bytes (in production, encrypt with app_password derived key)
    let priv_key_bytes = signing_key.to_bytes();
    webdav_upload(
        nextcloud_url,
        username,
        app_password,
        ".gradesta/identity.key",
        &priv_key_bytes,
    )?;

    // Create public share for identity.pub
    let share_url = create_public_share(
        nextcloud_url,
        username,
        app_password,
        "/.gradesta/identity.pub",
    )?;

    Ok((signing_key, share_url))
}

/// Load signing key from Nextcloud
pub fn load_signing_key(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
) -> Result<SigningKey> {
    let key_bytes = webdav_download(
        nextcloud_url,
        username,
        app_password,
        ".gradesta/identity.key",
    )?;

    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow!("Invalid key length"))?;

    SigningKey::from_bytes(&key_array.into()).map_err(|e| anyhow!("Invalid key: {}", e))
}
