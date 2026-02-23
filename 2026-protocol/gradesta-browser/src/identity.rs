use anyhow::{anyhow, Context, Result};
use p256::ecdsa::{signature::Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Stored identity for a Nextcloud account
///
/// The real identity is the `share_url` - the public URL where the public key is stored.
/// This proves control of that URL. The `display_name` is an unverified user-chosen label.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    /// User-chosen display name (unverified claim, can be anything)
    pub display_name: String,
    /// The Nextcloud server URL (for WebDAV access)
    pub nextcloud_url: String,
    /// Username on the Nextcloud server
    pub username: String,
    /// App password for authentication
    pub app_password: String,
    /// Public share URL for identity.pub - THIS IS THE REAL IDENTITY
    /// Proves control of this URL
    pub share_url: String,
    /// Server URLs that user has chosen to remember for auto-identification
    #[serde(default)]
    pub remembered_servers: Vec<String>,
    /// Signing key loaded at runtime
    #[serde(skip)]
    pub signing_key: Option<SigningKey>,
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

/// Serialize public key with identity metadata
/// Format: identity_string (null-terminated) + public key (65 bytes: 0x04 || X || Y)
pub fn serialize_public_key_with_identity(key: &VerifyingKey, username: &str, server: &str) -> Vec<u8> {
    use p256::EncodedPoint;
    let point: EncodedPoint = key.to_encoded_point(false);

    // Format: username@server\0 + 65-byte public key
    let identity = format!("{}@{}", username, server.replace("https://", "").replace("http://", ""));
    let mut data = Vec::with_capacity(identity.len() + 1 + 65);
    data.extend_from_slice(identity.as_bytes());
    data.push(0); // null terminator
    data.extend_from_slice(point.as_bytes());
    data
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
        "{}/ocs/v2.php/apps/files_sharing/api/v1/shares?format=json",
        nextcloud_url.trim_end_matches('/')
    );

    #[derive(Debug, Deserialize)]
    struct ShareResponse {
        ocs: OcsWrapper,
    }

    #[derive(Debug, Deserialize)]
    struct OcsWrapper {
        data: ShareData,
    }

    #[derive(Debug, Deserialize)]
    struct ShareData {
        url: String,
    }

    let resp = client
        .post(&url)
        .basic_auth(username, Some(app_password))
        .header("OCS-APIRequest", "true")
        .header("Accept", "application/json")
        .form(&[
            ("path", path),
            ("shareType", "3"), // 3 = public link
            ("permissions", "1"), // 1 = read
        ])
        .send()
        .context("Failed to create share")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(anyhow!("Share creation failed: {} - {}", status, body));
    }

    let body = resp.text().context("Failed to read share response body")?;
    eprintln!("Share response: {}", body);

    let share: ShareResponse = serde_json::from_str(&body).context("Failed to parse share response JSON")?;
    Ok(share.ocs.data.url)
}

/// Sign a challenge (nonce || timestamp) with the signing key
pub fn sign_challenge(signing_key: &SigningKey, nonce: &[u8], timestamp: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(nonce.len() + 8);
    data.extend_from_slice(nonce);
    data.extend_from_slice(&timestamp.to_be_bytes());

    // Sign the data (Signer::sign will hash it with SHA256 internally)
    let signature: p256::ecdsa::Signature = signing_key.sign(&data);
    signature.to_bytes().to_vec()
}

/// Complete identity setup: generate keys, upload to Nextcloud, create share
/// Also saves initial metadata with display_name
pub fn setup_identity(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
    display_name: &str,
) -> Result<(SigningKey, String)> {
    // Generate keypair
    let (signing_key, verifying_key) = generate_keypair();

    // Serialize public key with identity metadata
    let pub_key_bytes = serialize_public_key_with_identity(&verifying_key, username, nextcloud_url);

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

    // Save identity metadata to Nextcloud
    let metadata = IdentityMetadata {
        display_name: display_name.to_string(),
        share_url: share_url.clone(),
        remembered_servers: Vec::new(),
    };
    upload_identity_metadata(nextcloud_url, username, app_password, &metadata)?;

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

/// Identity metadata stored on Nextcloud (synced across devices)
/// Stored at .gradesta/identity.toml
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IdentityMetadata {
    /// User-chosen display name
    pub display_name: String,
    /// The public share URL for identity.pub
    pub share_url: String,
    /// Server URLs that user has chosen to remember for auto-identification
    #[serde(default)]
    pub remembered_servers: Vec<String>,
}

/// Upload identity metadata to Nextcloud
pub fn upload_identity_metadata(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
    metadata: &IdentityMetadata,
) -> Result<()> {
    let content = toml::to_string_pretty(metadata).context("Failed to serialize metadata")?;
    webdav_upload(
        nextcloud_url,
        username,
        app_password,
        ".gradesta/identity.toml",
        content.as_bytes(),
    )
}

/// Download identity metadata from Nextcloud (returns None if not found)
pub fn download_identity_metadata(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
) -> Result<Option<IdentityMetadata>> {
    match webdav_download(nextcloud_url, username, app_password, ".gradesta/identity.toml") {
        Ok(data) => {
            let content = String::from_utf8(data).context("Invalid UTF-8 in metadata")?;
            let metadata: IdentityMetadata = toml::from_str(&content).context("Failed to parse metadata")?;
            Ok(Some(metadata))
        }
        Err(e) => {
            // Check if it's a 404 (file not found)
            let err_str = e.to_string();
            if err_str.contains("404") || err_str.contains("Not Found") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Sync identity from Nextcloud - loads key and metadata
/// Returns (signing_key, metadata) if identity exists, or None if no identity on server
pub fn sync_identity_from_nextcloud(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
) -> Result<Option<(SigningKey, IdentityMetadata)>> {
    // Try to load signing key
    let signing_key = match load_signing_key(nextcloud_url, username, app_password) {
        Ok(key) => key,
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("404") || err_str.contains("Not Found") {
                return Ok(None);
            }
            return Err(e);
        }
    };

    // Try to load metadata
    let metadata = download_identity_metadata(nextcloud_url, username, app_password)?
        .unwrap_or_default();

    Ok(Some((signing_key, metadata)))
}

/// Bidirectional sync of identity metadata
/// Merges local and remote remembered_servers, uploads merged result
/// Returns the merged metadata
pub fn sync_identity_bidirectional(
    nextcloud_url: &str,
    username: &str,
    app_password: &str,
    local: &IdentityMetadata,
) -> Result<IdentityMetadata> {
    // Download remote metadata
    let remote = download_identity_metadata(nextcloud_url, username, app_password)?
        .unwrap_or_default();

    // Merge remembered_servers (union of both lists)
    let mut merged_servers: Vec<String> = local.remembered_servers.clone();
    for server in &remote.remembered_servers {
        if !merged_servers.contains(server) {
            merged_servers.push(server.clone());
        }
    }

    // Use local display_name and share_url (they shouldn't change)
    // But if local is empty and remote has values, use remote
    let merged = IdentityMetadata {
        display_name: if local.display_name.is_empty() {
            remote.display_name
        } else {
            local.display_name.clone()
        },
        share_url: if local.share_url.is_empty() {
            remote.share_url
        } else {
            local.share_url.clone()
        },
        remembered_servers: merged_servers,
    };

    // Upload merged metadata back to Nextcloud
    upload_identity_metadata(nextcloud_url, username, app_password, &merged)?;

    Ok(merged)
}
