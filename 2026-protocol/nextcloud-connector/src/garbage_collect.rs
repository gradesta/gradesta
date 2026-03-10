//! Garbage collection for content-addressable storage
//!
//! Removes content files that are no longer referenced by any commit in git history.
//! This is safe because:
//! 1. Content files are immutable (never modified, only created)
//! 2. Old commits reference old hashes
//! 3. GC only runs when explicitly triggered
//! 4. We scan ALL commits before deleting anything

use anyhow::Result;
use std::collections::HashSet;

use crate::content_store::{LocalContentStore, CONTENT_STORE_DIR};
use crate::git_undo::GitUndoRepo;
use crate::notes::NotesIndex;

/// Statistics from a garbage collection run
#[derive(Debug, Default)]
pub struct GcStats {
    /// Number of content files found in the store
    pub total_files: usize,
    /// Number of hashes referenced by git history
    pub referenced_hashes: usize,
    /// Number of files deleted
    pub deleted_files: usize,
    /// Bytes freed
    pub bytes_freed: u64,
}

/// Garbage collector for content-addressable storage
pub struct GarbageCollector<'a> {
    git_repo: &'a GitUndoRepo,
    content_store: LocalContentStore,
}

impl<'a> GarbageCollector<'a> {
    /// Create a new garbage collector
    pub fn new(git_repo: &'a GitUndoRepo) -> Self {
        let workdir = git_repo.workdir().expect("Git repo has no working directory");
        Self {
            git_repo,
            content_store: LocalContentStore::new(workdir),
        }
    }

    /// Collect all content hashes referenced by any commit in git history
    ///
    /// This parses the index.toml from each commit to find referenced hashes.
    pub fn collect_referenced_hashes(&self) -> Result<HashSet<String>> {
        let mut referenced = HashSet::new();

        let commits = self.git_repo.get_all_commits()?;
        log::info!("GC: Scanning {} commits for referenced hashes", commits.len());

        for commit_info in &commits {
            // Checkout the commit to read its index.toml
            // Note: We could also use git show to read files without checkout,
            // but for simplicity we'll parse from working directory
            if let Ok(index) = self.read_index_at_commit(commit_info.oid) {
                self.collect_hashes_from_index(&index, &mut referenced);
            }
        }

        log::info!("GC: Found {} unique referenced hashes", referenced.len());
        Ok(referenced)
    }

    /// Read the notes index from a specific commit
    fn read_index_at_commit(&self, oid: git2::Oid) -> Result<NotesIndex> {
        let workdir = self.git_repo.workdir()
            .ok_or_else(|| anyhow::anyhow!("No working directory"))?;

        // Save current HEAD
        let current_head = self.git_repo.head_commit();

        // Checkout the commit
        self.git_repo.checkout_commit(oid)?;

        // Read the index
        let result = NotesIndex::load_from_path(workdir);

        // Restore original HEAD if we had one
        if let Some(head_oid) = current_head {
            let _ = self.git_repo.checkout_commit(head_oid);
        }

        result
    }

    /// Extract all content hashes from an index
    fn collect_hashes_from_index(&self, index: &NotesIndex, hashes: &mut HashSet<String>) {
        for vertex in &index.vertices {
            // Check content_hash field
            if !vertex.content_hash.is_empty() {
                hashes.insert(vertex.content_hash.clone());
            }

            // Check layers
            for layer_content in vertex.layers.values() {
                if !layer_content.content_hash.is_empty() {
                    hashes.insert(layer_content.content_hash.clone());
                }
            }
        }
    }

    /// Run garbage collection
    ///
    /// This collects all referenced hashes from git history,
    /// then deletes any content files not in that set.
    pub fn gc(&self) -> Result<GcStats> {
        let mut stats = GcStats::default();

        // Get all referenced hashes
        let referenced = self.collect_referenced_hashes()?;
        stats.referenced_hashes = referenced.len();

        // List all files in content store
        let all_hashes = self.content_store.list_hashes()?;
        stats.total_files = all_hashes.len();

        log::info!("GC: {} total files, {} referenced", stats.total_files, stats.referenced_hashes);

        // Delete unreferenced files
        for (hash, ext) in &all_hashes {
            if !referenced.contains(hash) {
                // Get file size before deletion for stats
                let workdir = self.git_repo.workdir().expect("No workdir");
                let path = workdir.join(CONTENT_STORE_DIR).join(format!("{}.{}", hash, ext));
                if let Ok(metadata) = std::fs::metadata(&path) {
                    stats.bytes_freed += metadata.len();
                }

                if let Err(e) = self.content_store.delete(hash, ext) {
                    log::warn!("GC: Failed to delete {}.{}: {}", hash, ext, e);
                } else {
                    stats.deleted_files += 1;
                    log::debug!("GC: Deleted {}.{}", hash, ext);
                }
            }
        }

        log::info!("GC: Deleted {} files, freed {} bytes",
                  stats.deleted_files, stats.bytes_freed);

        Ok(stats)
    }

    /// Count orphaned files without deleting them
    ///
    /// Returns the number of files that would be deleted by gc().
    pub fn count_orphaned(&self) -> Result<usize> {
        let referenced = self.collect_referenced_hashes()?;
        let all_hashes = self.content_store.list_hashes()?;

        let orphaned = all_hashes.iter()
            .filter(|(hash, _)| !referenced.contains(hash))
            .count();

        Ok(orphaned)
    }
}

/// Check if garbage collection should be triggered
///
/// Returns true if there are more than `threshold` orphaned files.
pub fn should_run_gc(git_repo: &GitUndoRepo, threshold: usize) -> bool {
    let gc = GarbageCollector::new(git_repo);
    match gc.count_orphaned() {
        Ok(count) => {
            log::debug!("GC check: {} orphaned files, threshold {}", count, threshold);
            count > threshold
        }
        Err(e) => {
            log::warn!("GC check failed: {}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_gc_stats_default() {
        let stats = GcStats::default();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.referenced_hashes, 0);
        assert_eq!(stats.deleted_files, 0);
        assert_eq!(stats.bytes_freed, 0);
    }
}
