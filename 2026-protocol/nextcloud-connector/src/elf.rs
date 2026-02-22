//! Elf handling module for the Nextcloud connector
//!
//! This module manages elf connections, invitations, and message routing.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::protocol::{
    Invitation, RegionSpec,
    PERM_READ, PERM_WRITE, PERM_CREATE, PERM_DELETE,
};

/// Generate a secure random token for elf invitations
pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Registry of pending elf invitations
#[derive(Default)]
pub struct ElfRegistry {
    /// Token -> Invitation mapping
    invitations: HashMap<String, Invitation>,
    /// Token -> Browser connection ID (for forwarding output)
    browser_connections: HashMap<String, u64>,
}

impl ElfRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new invitation and return the token
    pub fn create_invitation(
        &mut self,
        summoner_identity: String,
        elf_url: String,
        region: RegionSpec,
        permissions: u8,
        command: String,
        params: HashMap<String, String>,
        browser_conn_id: u64,
        ttl_seconds: i64,
    ) -> String {
        let token = generate_token();
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64 + ttl_seconds)
            .unwrap_or(0);

        let invitation = Invitation {
            token: token.clone(),
            summoner_identity,
            elf_url,
            region,
            permissions,
            expires_at,
            command,
            params,
        };

        self.invitations.insert(token.clone(), invitation);
        self.browser_connections.insert(token.clone(), browser_conn_id);
        token
    }

    /// Validate and retrieve an invitation by token
    pub fn validate_token(&self, token: &str) -> Result<&Invitation> {
        let invitation = self.invitations.get(token)
            .ok_or_else(|| anyhow!("Invalid invitation token"))?;

        if invitation.is_expired() {
            return Err(anyhow!("Invitation has expired"));
        }

        Ok(invitation)
    }

    /// Consume an invitation (remove from pending)
    pub fn consume_invitation(&mut self, token: &str) -> Option<Invitation> {
        self.invitations.remove(token)
    }

    /// Get the browser connection ID for forwarding output
    pub fn get_browser_connection(&self, token: &str) -> Option<u64> {
        self.browser_connections.get(token).copied()
    }

    /// Remove browser connection mapping
    pub fn remove_browser_connection(&mut self, token: &str) {
        self.browser_connections.remove(token);
    }

    /// Clean up expired invitations
    pub fn cleanup_expired(&mut self) {
        let expired_tokens: Vec<String> = self.invitations.iter()
            .filter(|(_, inv)| inv.is_expired())
            .map(|(token, _)| token.clone())
            .collect();

        for token in expired_tokens {
            self.invitations.remove(&token);
            self.browser_connections.remove(&token);
        }
    }

    /// Clean up all invitations associated with a browser connection
    /// Called when a browser disconnects
    pub fn cleanup_for_browser(&mut self, browser_conn_id: u64) {
        let tokens_to_remove: Vec<String> = self.browser_connections.iter()
            .filter(|(_, &id)| id == browser_conn_id)
            .map(|(token, _)| token.clone())
            .collect();

        for token in tokens_to_remove {
            self.invitations.remove(&token);
            self.browser_connections.remove(&token);
        }
    }
}

/// Active elf connection state
pub struct ElfConnection {
    /// The invitation this elf connected with
    pub invitation: Invitation,
    /// Browser connection ID to forward output to
    pub browser_conn_id: u64,
    /// Set of vertices the elf has accessed (for scope validation)
    pub accessed_vertices: std::collections::HashSet<u64>,
}

impl ElfConnection {
    pub fn new(invitation: Invitation, browser_conn_id: u64) -> Self {
        Self {
            invitation,
            browser_conn_id,
            accessed_vertices: std::collections::HashSet::new(),
        }
    }

    /// Check if the elf can access a vertex
    /// For now, we just check if the vertex is in the allowed region
    /// TODO: Implement proper graph traversal validation
    pub fn can_access_vertex(&self, vertex_id: u64) -> bool {
        // Origin vertex is always accessible
        if vertex_id == self.invitation.region.origin_vertex {
            return true;
        }
        // For now, allow access to any vertex that was previously accessed
        // or if it's adjacent to an accessed vertex
        // Full implementation would track graph traversal
        self.accessed_vertices.contains(&vertex_id) ||
            self.accessed_vertices.is_empty() // First access
    }

    /// Record that a vertex was accessed
    pub fn record_access(&mut self, vertex_id: u64) {
        self.accessed_vertices.insert(vertex_id);
    }

    /// Check read permission
    pub fn can_read(&self) -> bool {
        self.invitation.has_permission(PERM_READ)
    }

    /// Check write permission
    pub fn can_write(&self) -> bool {
        self.invitation.has_permission(PERM_WRITE)
    }

    /// Check create permission
    pub fn can_create(&self) -> bool {
        self.invitation.has_permission(PERM_CREATE)
    }

    /// Check delete permission
    pub fn can_delete(&self) -> bool {
        self.invitation.has_permission(PERM_DELETE)
    }
}

/// Thread-safe elf registry wrapper
pub type SharedElfRegistry = Arc<Mutex<ElfRegistry>>;

/// Create a new shared elf registry
pub fn new_shared_registry() -> SharedElfRegistry {
    Arc::new(Mutex::new(ElfRegistry::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let token1 = generate_token();
        let token2 = generate_token();
        assert_ne!(token1, token2);
        assert!(!token1.is_empty());
    }

    #[test]
    fn test_invitation_lifecycle() {
        let mut registry = ElfRegistry::new();

        let region = RegionSpec::new("test://landmark", 12345, 0xFF, -1);
        let token = registry.create_invitation(
            "user@example.com".to_string(),
            "http://localhost:9000".to_string(),
            region,
            PERM_READ | PERM_WRITE,
            "code".to_string(),
            HashMap::new(),
            1,
            3600,
        );

        // Token should be valid
        assert!(registry.validate_token(&token).is_ok());

        // Browser connection should be tracked
        assert_eq!(registry.get_browser_connection(&token), Some(1));

        // Consume the invitation
        let invitation = registry.consume_invitation(&token).unwrap();
        assert_eq!(invitation.command, "code");

        // Token should no longer be valid
        assert!(registry.validate_token(&token).is_err());
    }
}
