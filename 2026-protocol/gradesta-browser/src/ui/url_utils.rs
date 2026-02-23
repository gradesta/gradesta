//! URL parsing utilities for the split URL bar

/// Result of parsing a full Gradesta URL
pub struct ParsedUrl {
    pub server: String,   // ws://host:port
    pub landmark: String, // /path/to/landmark
}

/// Detect if text looks like a full URL (has protocol and query param)
pub fn looks_like_full_url(text: &str) -> bool {
    let text = text.trim();
    (text.starts_with("ws://") || text.starts_with("wss://")) && text.contains("?landmark=")
}

/// Parse a full URL into server and landmark parts
pub fn parse_full_url(url_str: &str) -> Option<ParsedUrl> {
    let url_str = url_str.trim();
    if !looks_like_full_url(url_str) {
        return None;
    }

    if let Ok(parsed) = url::Url::parse(url_str) {
        let port = parsed
            .port()
            .unwrap_or(if parsed.scheme() == "wss" { 443 } else { 80 });
        let server = format!(
            "{}://{}:{}",
            parsed.scheme(),
            parsed.host_str().unwrap_or("localhost"),
            port
        );

        let landmark = parsed
            .query_pairs()
            .find(|(k, _)| k == "landmark")
            .map(|(_, v)| v.to_string())
            .unwrap_or_else(|| "/".to_string());

        Some(ParsedUrl { server, landmark })
    } else {
        None
    }
}

/// Construct a full URL from server and landmark for connection
pub fn construct_full_url(server: &str, landmark: &str) -> String {
    let server = server.trim();
    let landmark = landmark.trim();

    // Add /ws path for WebSocket connection
    format!("{}/ws?landmark={}", server, landmark)
}

/// Set clipboard text using arboard
pub fn set_clipboard_text(text: &str) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => clipboard.set_text(text.to_string()).is_ok(),
        Err(_) => false,
    }
}
