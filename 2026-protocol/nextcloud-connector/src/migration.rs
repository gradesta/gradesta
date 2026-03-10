//! Migration from file-based storage to Content-Addressable Storage (CAS)
//!
//! This module handles:
//! 1. Migrating existing vertices from file paths to content hashes
//! 2. Rewriting git history to remove binary content (only index.toml in commits)
//! 3. Cleaning up old content files after migration

use anyhow::{anyhow, Context, Result};
use git2::{Oid, Repository, Signature};
use std::collections::HashMap;
use std::path::Path;

use crate::content_store::{ContentStore, LocalContentStore, CONTENT_STORE_DIR};
use crate::git_undo::GitUndoRepo;
use crate::nextcloud::NextcloudClient;
use crate::notes::{mime_to_extension, NotesIndex, CONTENT_DIR};

/// Statistics from a migration run
#[derive(Debug, Default)]
pub struct MigrationStats {
    /// Number of vertices migrated
    pub vertices_migrated: usize,
    /// Number of layers migrated
    pub layers_migrated: usize,
    /// Number of commits rewritten
    pub commits_rewritten: usize,
    /// Bytes saved in git history (estimated)
    pub bytes_saved_estimate: u64,
    /// Old content files that can be deleted
    pub old_files_to_delete: Vec<String>,
}

/// Migrate a repository from file-based to CAS-based storage
pub struct Migrator<'a> {
    nc: &'a NextcloudClient,
    git_repo: &'a GitUndoRepo,
}

impl<'a> Migrator<'a> {
    pub fn new(nc: &'a NextcloudClient, git_repo: &'a GitUndoRepo) -> Self {
        Self { nc, git_repo }
    }

    /// Run full migration: migrate content and rewrite history
    pub async fn migrate_full(&self) -> Result<MigrationStats> {
        let mut stats = MigrationStats::default();

        // Step 1: Migrate current index to CAS
        log::info!("Migration: Step 1 - Migrating current index to CAS");
        self.migrate_current_index(&mut stats).await?;

        // Step 2: Rewrite git history to remove binary content
        log::info!("Migration: Step 2 - Rewriting git history");
        self.rewrite_git_history(&mut stats)?;

        // Step 3: Push rewritten history
        log::info!("Migration: Step 3 - Pushing rewritten history");
        if self.git_repo.has_remote() {
            // Force push since we rewrote history
            self.force_push_rewritten_history()?;
        }

        log::info!("Migration complete: {:?}", stats);
        Ok(stats)
    }

    /// Migrate only the current index (no history rewrite)
    pub async fn migrate_current_index(&self, stats: &mut MigrationStats) -> Result<()> {
        let workdir = self.git_repo.workdir()
            .ok_or_else(|| anyhow!("Git repo has no working directory"))?;

        // Load current index
        let mut index = NotesIndex::load_from_path(workdir)?;

        // Check if migration needed
        let needs_migration = index.vertices.iter().any(|v| {
            !v.file.is_empty() && v.content_hash.is_empty()
        });

        if !needs_migration {
            log::info!("Migration: Index already migrated, skipping");
            return Ok(());
        }

        let content_store = ContentStore::new(self.nc.clone());
        let local_store = LocalContentStore::new(workdir);

        // Migrate each vertex
        for vertex in &mut index.vertices {
            // Migrate primary content
            if !vertex.file.is_empty() && vertex.content_hash.is_empty() {
                match self.migrate_file(&vertex.file, &vertex.mime, &content_store, &local_store).await {
                    Ok(hash) => {
                        log::info!("Migrated {} -> {}", vertex.file, hash);
                        stats.old_files_to_delete.push(vertex.file.clone());
                        vertex.content_hash = hash;
                        vertex.file.clear();
                        stats.vertices_migrated += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to migrate {}: {}", vertex.file, e);
                    }
                }
            }

            // Migrate layers
            for (layer_id, layer) in &mut vertex.layers {
                if !layer.file.is_empty() && layer.content_hash.is_empty() {
                    match self.migrate_file(&layer.file, &layer.mime, &content_store, &local_store).await {
                        Ok(hash) => {
                            log::info!("Migrated layer {} {} -> {}", layer_id, layer.file, hash);
                            stats.old_files_to_delete.push(layer.file.clone());
                            layer.content_hash = hash;
                            layer.file.clear();
                            stats.layers_migrated += 1;
                        }
                        Err(e) => {
                            log::warn!("Failed to migrate layer {}: {}", layer.file, e);
                        }
                    }
                }
            }
        }

        // Bump version
        index.meta.version = crate::notes::CURRENT_INDEX_VERSION;

        // Save migrated index
        index.save(self.nc).await?;
        index.save_to_path(workdir)?;

        // Commit the migrated index
        self.git_repo.ensure_on_branch()?;
        self.git_repo.commit_all("Migrate to CAS format", "migration")?;

        Ok(())
    }

    /// Migrate a single file to CAS
    async fn migrate_file(
        &self,
        file_path: &str,
        mime: &str,
        content_store: &ContentStore<NextcloudClient>,
        local_store: &LocalContentStore,
    ) -> Result<String> {
        // Download content from old location
        let content = self.nc.download(file_path).await
            .with_context(|| format!("Failed to download {}", file_path))?;

        let ext = mime_to_extension(mime);

        // Upload to CAS (both remote and local)
        let hash = content_store.put(&content, ext).await?;
        local_store.put(&content, ext)?;

        Ok(hash)
    }

    /// Rewrite git history to remove all binary content
    ///
    /// This creates new commits that only contain index.toml,
    /// effectively removing all binary content from history.
    fn rewrite_git_history(&self, stats: &mut MigrationStats) -> Result<()> {
        let workdir = self.git_repo.workdir()
            .ok_or_else(|| anyhow!("Git repo has no working directory"))?;

        // Get all commits in order (oldest first)
        let commits = self.git_repo.get_all_commits()?;

        if commits.is_empty() {
            return Ok(());
        }

        // We'll rebuild history with only index.toml in each commit
        // Map from old OID to new OID
        let mut oid_map: HashMap<Oid, Oid> = HashMap::new();

        let repo = Repository::open(workdir)?;
        let sig = Signature::now("migration", "migration@gradesta")?;

        for commit_info in &commits {
            // Get the old commit
            let _old_commit = repo.find_commit(commit_info.oid)?;

            // Checkout this commit to get its index.toml
            self.git_repo.checkout_commit(commit_info.oid)?;

            // Read the index.toml content at this commit
            let index_path = workdir.join("index.toml");
            let index_content = if index_path.exists() {
                std::fs::read(&index_path)?
            } else {
                // No index.toml at this commit, create empty tree
                Vec::new()
            };

            // Create a new tree with only index.toml
            let mut tree_builder = repo.treebuilder(None)?;

            if !index_content.is_empty() {
                // Create blob for index.toml
                let blob_oid = repo.blob(&index_content)?;
                tree_builder.insert("index.toml", blob_oid, 0o100644)?;
            }

            let new_tree_oid = tree_builder.write()?;
            let new_tree = repo.find_tree(new_tree_oid)?;

            // Map parent OIDs to new OIDs
            let new_parents: Vec<Oid> = commit_info.parent_oids.iter()
                .filter_map(|old_parent| oid_map.get(old_parent).copied())
                .collect();

            let parent_commits: Vec<git2::Commit> = new_parents.iter()
                .filter_map(|oid| repo.find_commit(*oid).ok())
                .collect();

            let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();

            // Create new commit
            let new_oid = repo.commit(
                None, // Don't update any ref yet
                &sig,
                &sig,
                &commit_info.message,
                &new_tree,
                &parent_refs,
            )?;

            oid_map.insert(commit_info.oid, new_oid);
            stats.commits_rewritten += 1;

            // Estimate bytes saved (rough: assume binary content is ~10x index.toml size)
            stats.bytes_saved_estimate += (index_content.len() as u64) * 10;
        }

        // Update HEAD to point to the rewritten tip
        if let Some(last_commit) = commits.last() {
            if let Some(&new_tip) = oid_map.get(&last_commit.oid) {
                // Checkout the new tip
                let new_commit = repo.find_commit(new_tip)?;
                repo.checkout_tree(
                    new_commit.tree()?.as_object(),
                    Some(git2::build::CheckoutBuilder::new().force()),
                )?;

                // Update HEAD
                let head = repo.head()?;
                if head.is_branch() {
                    let branch_name = head.shorthand().unwrap_or("master");
                    repo.reference(
                        &format!("refs/heads/{}", branch_name),
                        new_tip,
                        true,
                        "Migration: rewrite history without binary content",
                    )?;
                    repo.set_head(&format!("refs/heads/{}", branch_name))?;
                } else {
                    repo.set_head_detached(new_tip)?;
                }

                log::info!("History rewritten: {} -> {}", last_commit.oid, new_tip);
            }
        }

        // Restore working directory to latest state
        self.restore_working_directory(workdir)?;

        // Run git gc to actually reclaim space
        self.run_git_gc(workdir)?;

        Ok(())
    }

    /// Restore the working directory with current index and content-store
    fn restore_working_directory(&self, workdir: &Path) -> Result<()> {
        // The index.toml should already be in place from the last checkout
        // Just ensure content-store directory exists
        let content_store_path = workdir.join(CONTENT_STORE_DIR);
        std::fs::create_dir_all(&content_store_path)?;
        Ok(())
    }

    /// Run git gc to reclaim space
    fn run_git_gc(&self, workdir: &Path) -> Result<()> {
        log::info!("Running git gc to reclaim space...");

        // Remove old refs that might keep objects alive
        let git_dir = workdir.join(".git");

        // Remove reflogs
        let reflog_dir = git_dir.join("logs");
        if reflog_dir.exists() {
            log::info!("Removing reflogs...");
            let _ = std::fs::remove_dir_all(&reflog_dir);
        }

        // Remove stale refs
        let refs_original = git_dir.join("refs").join("original");
        if refs_original.exists() {
            log::info!("Removing refs/original...");
            let _ = std::fs::remove_dir_all(&refs_original);
        }

        // Run git reflog expire
        log::info!("Expiring reflog...");
        let output = std::process::Command::new("git")
            .args(["reflog", "expire", "--expire=now", "--all"])
            .current_dir(workdir)
            .output();

        if let Err(e) = output {
            log::warn!("Failed to expire reflog: {}", e);
        }

        // Run git gc --aggressive --prune=now
        log::info!("Running git gc --aggressive --prune=now (this may take a while)...");
        let output = std::process::Command::new("git")
            .args(["gc", "--aggressive", "--prune=now"])
            .current_dir(workdir)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    log::info!("Git gc completed successfully");
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    log::warn!("Git gc finished with warnings: {}", stderr);
                }
            }
            Err(e) => {
                log::warn!("Failed to run git gc: {}", e);
                log::info!("You may need to run 'git gc --aggressive --prune=now' manually");
            }
        }

        Ok(())
    }

    /// Force push rewritten history to remote
    fn force_push_rewritten_history(&self) -> Result<()> {
        let workdir = self.git_repo.workdir()
            .ok_or_else(|| anyhow!("Git repo has no working directory"))?;

        let repo = Repository::open(workdir)?;
        let mut remote = repo.find_remote("origin")?;

        let head = repo.head()?;
        let branch_name = if head.is_branch() {
            head.shorthand().unwrap_or("master").to_string()
        } else {
            "master".to_string()
        };

        // Force push
        let refspec = format!("+refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        remote.push(&[&refspec], None)
            .context("Failed to force push rewritten history")?;

        log::info!("Force pushed rewritten history to remote");
        Ok(())
    }

    /// Delete old content files after successful migration
    pub async fn cleanup_old_files(&self, files: &[String]) -> Result<usize> {
        let mut deleted = 0;

        for file_path in files {
            // Only delete files in the old content directory
            if file_path.starts_with(CONTENT_DIR) {
                match self.nc.delete(file_path).await {
                    Ok(()) => {
                        log::info!("Deleted old file: {}", file_path);
                        deleted += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to delete {}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(deleted)
    }
}

/// Run automatic migration for a repository
/// This is a standalone function that owns its data, suitable for async contexts
/// where we can't hold references to git2::Repository across await points.
pub async fn auto_migrate(workdir: std::path::PathBuf, nc: NextcloudClient) -> Result<MigrationStats> {
    let mut stats = MigrationStats::default();

    // Load current index
    let mut index = NotesIndex::load_from_path(&workdir)?;

    // Check if migration needed
    let needs = index.vertices.iter().any(|v| {
        !v.file.is_empty() && v.content_hash.is_empty()
    });

    if !needs {
        log::info!("Auto-migration: Index already migrated, skipping");
        return Ok(stats);
    }

    log::info!("Auto-migration: Starting migration for {} vertices", index.vertices.len());

    let content_store = ContentStore::new(nc.clone());
    let local_store = LocalContentStore::new(&workdir);

    // Migrate each vertex
    for vertex in &mut index.vertices {
        // Migrate primary content
        if !vertex.file.is_empty() && vertex.content_hash.is_empty() {
            match migrate_single_file(&nc, &vertex.file, &vertex.mime, &content_store, &local_store).await {
                Ok(hash) => {
                    log::info!("Auto-migrated {} -> {}", vertex.file, hash);
                    stats.old_files_to_delete.push(vertex.file.clone());
                    vertex.content_hash = hash;
                    vertex.file.clear();
                    stats.vertices_migrated += 1;
                }
                Err(e) => {
                    log::warn!("Failed to auto-migrate {}: {}", vertex.file, e);
                }
            }
        }

        // Migrate layers
        for (layer_id, layer) in &mut vertex.layers {
            if !layer.file.is_empty() && layer.content_hash.is_empty() {
                match migrate_single_file(&nc, &layer.file, &layer.mime, &content_store, &local_store).await {
                    Ok(hash) => {
                        log::info!("Auto-migrated layer {} {} -> {}", layer_id, layer.file, hash);
                        stats.old_files_to_delete.push(layer.file.clone());
                        layer.content_hash = hash;
                        layer.file.clear();
                        stats.layers_migrated += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to auto-migrate layer {}: {}", layer.file, e);
                    }
                }
            }
        }
    }

    // Bump version
    index.meta.version = crate::notes::CURRENT_INDEX_VERSION;

    // Save migrated index
    index.save(&nc).await?;
    index.save_to_path(&workdir)?;

    // Commit the migrated index using a fresh Repository
    // (we can do this synchronously at the end since all async work is done)
    let repo = Repository::open(&workdir)?;
    let sig = Signature::now("auto-migration", "auto-migration@gradesta")?;
    let mut git_index = repo.index()?;
    git_index.add_all(["index.toml"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    git_index.write()?;
    let tree_oid = git_index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "Auto-migrate to CAS format",
        &tree,
        &parents,
    )?;

    log::info!("Auto-migration complete: {:?}", stats);
    Ok(stats)
}

/// Migrate a single file to CAS (helper for auto_migrate)
async fn migrate_single_file(
    nc: &NextcloudClient,
    file_path: &str,
    mime: &str,
    content_store: &ContentStore<NextcloudClient>,
    local_store: &LocalContentStore,
) -> Result<String> {
    // Download content from old location
    let content = nc.download(file_path).await
        .with_context(|| format!("Failed to download {}", file_path))?;

    let ext = mime_to_extension(mime);

    // Upload to CAS (both remote and local)
    let hash = content_store.put(&content, ext).await?;
    local_store.put(&content, ext)?;

    Ok(hash)
}

/// Check if migration is needed for an index
pub fn needs_migration(index: &NotesIndex) -> bool {
    // Check if any vertex uses old file-based format
    index.vertices.iter().any(|v| {
        (!v.file.is_empty() && v.content_hash.is_empty()) ||
        v.layers.values().any(|l| !l.file.is_empty() && l.content_hash.is_empty())
    })
}

/// Estimate space savings from migration
pub fn estimate_savings(index: &NotesIndex) -> u64 {
    let mut bytes = 0u64;

    for vertex in &index.vertices {
        if !vertex.file.is_empty() && vertex.content_hash.is_empty() {
            // Rough estimate: average file size of 100KB
            bytes += 100 * 1024;
        }
        for layer in vertex.layers.values() {
            if !layer.file.is_empty() && layer.content_hash.is_empty() {
                bytes += 100 * 1024;
            }
        }
    }

    // Multiply by number of commits (each commit stored full copy before)
    // Rough estimate: 10 commits average
    bytes * 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_migration_empty() {
        let index = NotesIndex::default();
        assert!(!needs_migration(&index));
    }
}
