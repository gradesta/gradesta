//! Gradesta identity verification

use anyhow::{anyhow, Context, Result};
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

use crate::protocol;

/// Pending authentication state
#[derive(Debug, Clone)]
pub struct PendingAuth {
    pub nonce: [u8; 32],
    pub timestamp: i64,
    pub action_id: u64,
}

/// Public key data fetched from identity URL
#[derive(Debug)]
pub struct PublicKeyData {
    /// Embedded claim (username@server) - unverified, only used for Nextcloud URL derivation
    pub embedded_claim: String,
    pub key: VerifyingKey,
}

impl PendingAuth {
    /// Create new pending auth with random nonce
    pub fn new(action_id: u64) -> Self {
        let mut nonce = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        Self {
            nonce,
            timestamp,
            action_id,
        }
    }

    /// Encode the request identification message
    pub fn encode_request(&self, reason: &str) -> Vec<u8> {
        protocol::encode_request_identification(self.action_id, &self.nonce, self.timestamp, reason)
    }
}

/// Verified identity result
#[derive(Debug)]
pub struct VerifiedIdentity {
    /// The identity URL (the real identity - where the public key is hosted)
    pub identity_url: String,
    /// The embedded claim from the public key (username@server)
    pub embedded_claim: String,
}

/// Verify an identification response
pub async fn verify_identification(
    pending: &PendingAuth,
    action_id: u64,
    identity_url: &str,
    signature: &[u8],
) -> Result<VerifiedIdentity> {
    // Check action ID matches
    if action_id != pending.action_id {
        return Err(anyhow!(
            "Action ID mismatch: got {}, expected {}",
            action_id,
            pending.action_id
        ));
    }

    // Check timestamp is within ±5 minutes
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    if pending.timestamp < now - 300 || pending.timestamp > now + 300 {
        return Err(anyhow!(
            "Timestamp out of range: {} (now: {})",
            pending.timestamp,
            now
        ));
    }

    // Parse identity URL to get hosting server
    let parsed_url = Url::parse(identity_url).context("Failed to parse identity URL")?;
    let hosting_server = parsed_url
        .host_str()
        .ok_or_else(|| anyhow!("No host in identity URL"))?;

    // Fetch public key
    let pub_key_data = fetch_public_key(identity_url).await?;

    log::info!("Fetched public key with embedded claim: {}", pub_key_data.embedded_claim);

    // Parse embedded identity claim (format: username@server)
    // This is just a claim - the real identity is the URL where the key is hosted
    let parts: Vec<&str> = pub_key_data.embedded_claim.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid embedded identity format: {}", pub_key_data.embedded_claim));
    }
    let claimed_username = parts[0];
    let claimed_server = parts[1];

    // Verify claimed server matches hosting server
    // This ensures someone can't claim to be user@example.com while hosting on evil.com
    if !claimed_server.eq_ignore_ascii_case(hosting_server) {
        return Err(anyhow!(
            "Identity server mismatch: claimed {} but hosted on {}",
            claimed_server,
            hosting_server
        ));
    }

    // Verify signature
    // Signed data: nonce (32) || timestamp (8)
    // Note: p256's Signer::sign hashes internally, so we verify against the raw data
    let mut signed_data = Vec::with_capacity(40);
    signed_data.extend_from_slice(&pending.nonce);
    signed_data.extend_from_slice(&(pending.timestamp as u64).to_be_bytes());

    // Parse signature (r || s, 32 bytes each)
    if signature.len() != 64 {
        return Err(anyhow!("Invalid signature length: {} (expected 64)", signature.len()));
    }

    let sig = Signature::from_slice(signature).context("Failed to parse signature")?;

    // Verify against raw data - the Verifier trait will hash it internally
    pub_key_data
        .key
        .verify(&signed_data, &sig)
        .map_err(|e| anyhow!("Signature verification failed: {:?}", e))?;

    log::info!("Signature verified successfully!");
    log::info!("Identity URL (real identity): {}", identity_url);
    log::info!("Embedded claim (verified): {}", pub_key_data.embedded_claim);

    // Return both the URL and the embedded claim
    Ok(VerifiedIdentity {
        identity_url: identity_url.to_string(),
        embedded_claim: pub_key_data.embedded_claim,
    })
}

/// Fetch public key from identity URL
async fn fetch_public_key(identity_url: &str) -> Result<PublicKeyData> {
    let download_url = format!("{}/download", identity_url);

    let response = reqwest::get(&download_url)
        .await
        .context("Failed to fetch public key")?;

    if !response.status().is_success() {
        return Err(anyhow!("HTTP {}", response.status()));
    }

    let key_data = response.bytes().await.context("Failed to read response")?;

    // Parse: identity_string (null-terminated) + public key (65 bytes: 0x04 || X || Y)
    let null_idx = key_data
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No identity found in public key file"))?;

    let identity = String::from_utf8(key_data[..null_idx].to_vec())
        .context("Invalid identity string")?;
    let key_bytes = &key_data[null_idx + 1..];

    // Parse uncompressed point format (0x04 || X || Y)
    if key_bytes.len() != 65 || key_bytes[0] != 0x04 {
        return Err(anyhow!(
            "Invalid public key format (len={})",
            key_bytes.len()
        ));
    }

    let verifying_key = VerifyingKey::from_sec1_bytes(key_bytes)
        .context("Failed to parse public key")?;

    Ok(PublicKeyData {
        embedded_claim: identity,
        key: verifying_key,
    })
}
