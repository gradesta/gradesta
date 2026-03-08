//! Utility functions for the Nextcloud Connector

/// Hash a string to u64
pub fn hash_string(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Generate hash for router vertices (matches router.rs)
pub fn router_hash(identity: &str, name: &str) -> u64 {
    hash_string(&format!("router:{}:{}", identity, name))
}
