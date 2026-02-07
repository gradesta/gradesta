//! HTTP streaming server for large file downloads
//!
//! Provides secure one-time download links for large files (videos, etc.)
//! using JWT tokens with time-limited validity.

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::nextcloud::NextcloudClient;

/// JWT secret key - generated at server startup
#[derive(Clone)]
pub struct JwtSecret(pub [u8; 32]);

impl Default for JwtSecret {
    fn default() -> Self {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        Self(key)
    }
}

/// JWT claims for streaming tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamClaims {
    /// Nextcloud file path
    pub path: String,
    /// Expected MIME type
    pub mime: String,
    /// Nextcloud server URL
    pub server: String,
    /// Nextcloud username
    pub username: String,
    /// Nextcloud app password (encrypted in future)
    pub password: String,
    /// Token expiration (Unix timestamp)
    pub exp: usize,
    /// Token issued at (Unix timestamp)
    pub iat: usize,
}

/// Token TTL in seconds (1 hour)
const TOKEN_TTL_SECS: i64 = 3600;

/// Generate a streaming token for a file
pub fn generate_token(
    secret: &JwtSecret,
    path: &str,
    mime: &str,
    nc: &NextcloudClient,
) -> Result<String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let exp = now + TOKEN_TTL_SECS as usize;

    let claims = StreamClaims {
        path: path.to_string(),
        mime: mime.to_string(),
        server: nc.base_url.clone(),
        username: nc.username.clone(),
        password: nc.password.clone(),
        exp,
        iat: now,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret.0),
    )?;

    Ok(token)
}

/// Shared state for HTTP streaming handlers
#[derive(Clone)]
pub struct StreamState {
    pub jwt_secret: Arc<JwtSecret>,
}

/// Handle streaming request
pub async fn handle_stream(
    State(state): State<StreamState>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    // Decode and validate JWT
    let claims = match decode::<StreamClaims>(
        &token,
        &DecodingKey::from_secret(&state.jwt_secret.0),
        &Validation::default(),
    ) {
        Ok(data) => data.claims,
        Err(e) => {
            log::warn!("Invalid streaming token: {}", e);
            return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response();
        }
    };

    log::info!("Streaming file: {} ({})", claims.path, claims.mime);

    // Create Nextcloud client from claims
    let nc = NextcloudClient::new(&claims.server, &claims.username, &claims.password);

    // Check for Range header
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_range_header);

    // Get file info for Content-Length
    let file_size = match nc.get_file_size(&claims.path).await {
        Ok(size) => size,
        Err(e) => {
            log::error!("Failed to get file size: {}", e);
            return (StatusCode::NOT_FOUND, "File not found").into_response();
        }
    };

    // Handle range request
    let (start, end, status) = if let Some((req_start, req_end)) = range {
        let end = req_end.unwrap_or(file_size - 1).min(file_size - 1);
        let start = req_start.min(end);
        (start, end, StatusCode::PARTIAL_CONTENT)
    } else {
        (0, file_size - 1, StatusCode::OK)
    };

    let content_length = end - start + 1;

    // Stream file content
    match nc.stream_file(&claims.path, start, end).await {
        Ok(stream) => {
            let mut response = Response::builder()
                .status(status)
                .header(header::CONTENT_TYPE, &claims.mime)
                .header(header::CONTENT_LENGTH, content_length)
                .header(header::ACCEPT_RANGES, "bytes");

            if status == StatusCode::PARTIAL_CONTENT {
                response = response.header(
                    header::CONTENT_RANGE,
                    format!("bytes {}-{}/{}", start, end, file_size),
                );
            }

            response
                .body(Body::from_stream(stream))
                .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Stream error").into_response())
        }
        Err(e) => {
            log::error!("Failed to stream file: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to stream file").into_response()
        }
    }
}

/// Parse HTTP Range header (bytes=start-end)
fn parse_range_header(range: &str) -> Option<(u64, Option<u64>)> {
    let range = range.strip_prefix("bytes=")?;
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let start: u64 = parts[0].parse().ok()?;
    let end: Option<u64> = if parts[1].is_empty() {
        None
    } else {
        Some(parts[1].parse().ok()?)
    };

    Some((start, end))
}
