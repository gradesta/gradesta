//! Git-based undo system for notes
//!
//! Uses git2 (libgit2 Rust bindings) for undo history.
//! The git repository is stored in Nextcloud via WebDAV at `Notes/.gradesta-undo/`
//! and cloned to `/tmp/` on connect. Changes are pushed back to Nextcloud after
//! every commit for persistence across sessions and devices.
//!
//! Tree structure uses existing 6-directional edges:
//! - West: Parent commit (previous state)
//! - East: Child commit (next state on main branch)
//! - North/South: Sibling branches (alternative timelines)

use anyhow::{anyhow, Context, Result};
use git2::{
    BranchType, Commit, IndexAddOption, Oid, Repository, Signature,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::nextcloud::NextcloudClient;

/// Information about a commit for display/navigation
#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub oid: Oid,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
    pub parent_oids: Vec<Oid>,
}

/// Path in Nextcloud where the git repo is stored
pub const NEXTCLOUD_UNDO_PATH: &str = "Notes/.gradesta-undo";

/// Git repository wrapper for undo operations
pub struct GitUndoRepo {
    repo: Repository,
    local_path: PathBuf,
    /// Path to bare repo on mounted WebDAV (if using remote)
    remote_path: Option<PathBuf>,
}

/// Download git repo from Nextcloud and open it locally in /tmp
/// If no repo exists in Nextcloud, creates a new one
pub async fn open_from_nextcloud(nc: &NextcloudClient) -> Result<GitUndoRepo> {
    // Create a unique temp directory for this session
    let url_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        nc.url.hash(&mut hasher);
        nc.username.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };

    let local_path = PathBuf::from(format!("/tmp/gradesta-undo-{}", &url_hash[..8]));

    log::info!("Git undo: local path = {}", local_path.display());

    // Clean up any existing local directory to ensure fresh state
    if local_path.exists() {
        std::fs::remove_dir_all(&local_path)
            .context("Failed to clean up existing temp directory")?;
    }
    std::fs::create_dir_all(&local_path)
        .context("Failed to create temp directory")?;

    // Check if repo exists in Nextcloud
    let remote_git_path = format!("{}/.git", NEXTCLOUD_UNDO_PATH);
    let has_remote_repo = nc.exists(&format!("{}/HEAD", remote_git_path)).await;

    log::info!("Git undo: remote repo exists = {}", has_remote_repo);

    let repo = if has_remote_repo {
        // Download the git directory from Nextcloud
        download_git_dir(nc, &local_path).await?;
        Repository::open(&local_path).context("Failed to open downloaded git repository")?
    } else {
        // Initialize new repository
        let repo = Repository::init(&local_path).context("Failed to initialize git repository")?;

        // Create initial commit so we have a valid HEAD
        {
            let sig = Signature::now("gradesta", "gradesta@local")?;
            let tree_id = {
                let mut index = repo.index()?;
                index.write_tree()?
            };
            let tree = repo.find_tree(tree_id)?;
            repo.commit(Some("HEAD"), &sig, &sig, "Initial state", &tree, &[])?;
        }

        log::info!("Initialized new git repository");
        repo
    };

    let git_undo = GitUndoRepo { repo, local_path: local_path.clone(), remote_path: None };

    // If we created a new repo, upload it to Nextcloud
    if !has_remote_repo {
        sync_to_nextcloud(&local_path, nc).await?;
    }

    Ok(git_undo)
}

/// Download the .git directory from Nextcloud to local path
async fn download_git_dir(nc: &NextcloudClient, local_path: &Path) -> Result<()> {
    let remote_base = format!("{}/.git", NEXTCLOUD_UNDO_PATH);
    let local_git = local_path.join(".git");

    log::info!("Downloading git repo from Nextcloud: {}", remote_base);

    // List all files in the remote .git directory recursively
    download_dir_recursive(nc, &remote_base, &local_git).await?;

    log::info!("Git repo downloaded successfully");
    Ok(())
}

/// Recursively download a directory from Nextcloud
async fn download_dir_recursive(nc: &NextcloudClient, remote_path: &str, local_path: &Path) -> Result<()> {
    std::fs::create_dir_all(local_path)?;

    let entries = nc.list_directory(remote_path).await?;

    for entry in entries {
        let local_entry_path = local_path.join(&entry.name);
        let remote_entry_path = format!("{}/{}", remote_path, entry.name);

        if entry.is_directory {
            Box::pin(download_dir_recursive(nc, &remote_entry_path, &local_entry_path)).await?;
        } else {
            match nc.download(&remote_entry_path).await {
                Ok(content) => {
                    std::fs::write(&local_entry_path, &content)?;
                    log::debug!("Downloaded: {}", remote_entry_path);
                }
                Err(e) => {
                    log::warn!("Failed to download {}: {}", remote_entry_path, e);
                }
            }
        }
    }

    Ok(())
}

/// Sync the local git repo to Nextcloud
/// This uploads the .git directory to persist changes
pub async fn sync_to_nextcloud(local_path: &Path, nc: &NextcloudClient) -> Result<()> {
    let remote_base = format!("{}/.git", NEXTCLOUD_UNDO_PATH);
    let local_git = local_path.join(".git");

    log::info!("Uploading git repo to Nextcloud: {}", remote_base);

    // Create the base directory in Nextcloud
    nc.mkdir(NEXTCLOUD_UNDO_PATH).await?;
    nc.mkdir(&remote_base).await?;

    // Upload all files recursively
    upload_dir_recursive(nc, &local_git, &remote_base).await?;

    log::info!("Git repo uploaded successfully");
    Ok(())
}

/// Recursively upload a directory to Nextcloud
async fn upload_dir_recursive(nc: &NextcloudClient, local_path: &Path, remote_path: &str) -> Result<()> {
    let entries = std::fs::read_dir(local_path)?;

    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        let local_entry_path = entry.path();
        let remote_entry_path = format!("{}/{}", remote_path, file_name_str);

        if file_type.is_dir() {
            nc.mkdir(&remote_entry_path).await?;
            Box::pin(upload_dir_recursive(nc, &local_entry_path, &remote_entry_path)).await?;
        } else if file_type.is_file() {
            let content = std::fs::read(&local_entry_path)?;
            nc.upload(&remote_entry_path, &content).await?;
            log::debug!("Uploaded: {}", remote_entry_path);
        }
    }

    Ok(())
}

impl GitUndoRepo {
    /// Open or initialize a local repo with a mounted WebDAV remote
    ///
    /// This is the preferred method for production use - it uses git push/pull
    /// to a bare repo on the mounted WebDAV instead of manually uploading .git files.
    pub fn open_with_remote(local_path: &Path, remote_path: &Path) -> Result<Self> {
        log::info!("Git undo: opening with remote at {}", remote_path.display());

        // Clean up any existing local directory to ensure fresh state
        if local_path.exists() {
            std::fs::remove_dir_all(local_path)
                .context("Failed to clean up existing temp directory")?;
        }
        std::fs::create_dir_all(local_path)
            .context("Failed to create temp directory")?;

        // Check if remote bare repo exists
        let remote_head = remote_path.join("HEAD");
        let has_remote = remote_head.exists();

        log::info!("Git undo: remote bare repo exists = {}", has_remote);

        let repo = if has_remote {
            // Clone from the bare repo
            let remote_url = format!("file://{}", remote_path.display());
            log::info!("Git undo: cloning from {}", remote_url);

            Repository::clone(&remote_url, local_path)
                .context("Failed to clone from remote bare repo")?
        } else {
            // Initialize new local repository
            let repo = Repository::init(local_path)
                .context("Failed to initialize git repository")?;

            // Create initial commit so we have a valid HEAD
            {
                let sig = Signature::now("gradesta", "gradesta@local")?;
                let tree_id = {
                    let mut index = repo.index()?;
                    index.write_tree()?
                };
                let tree = repo.find_tree(tree_id)?;
                repo.commit(Some("HEAD"), &sig, &sig, "Initial state", &tree, &[])?;
            }

            // Initialize the remote bare repo
            Self::init_remote_bare(remote_path)?;

            // Add origin remote
            let remote_url = format!("file://{}", remote_path.display());
            repo.remote("origin", &remote_url)?;

            log::info!("Initialized new git repository with remote");
            repo
        };

        let git_undo = GitUndoRepo {
            repo,
            local_path: local_path.to_path_buf(),
            remote_path: Some(remote_path.to_path_buf()),
        };

        // If we just created a new repo, push to remote
        if !has_remote {
            git_undo.push()?;
        }

        Ok(git_undo)
    }

    /// Initialize a bare git repository at the given path
    pub fn init_remote_bare(remote_path: &Path) -> Result<()> {
        if !remote_path.join("HEAD").exists() {
            // Create parent directory if needed
            if let Some(parent) = remote_path.parent() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create parent directory for bare repo")?;
            }

            Repository::init_bare(remote_path)
                .context("Failed to initialize bare repository")?;
            log::info!("Initialized bare repo at {}", remote_path.display());
        }
        Ok(())
    }

    /// Push all branches to the remote
    ///
    /// This replaces sync_to_nextcloud() - uses efficient git push instead of
    /// manually uploading all .git files.
    pub fn push(&self) -> Result<()> {
        let remote_path = self.remote_path.as_ref()
            .ok_or_else(|| anyhow!("No remote configured"))?;

        log::info!("Git undo: pushing to {}", remote_path.display());

        let mut remote = self.repo.find_remote("origin")
            .context("Failed to find origin remote")?;

        // Get current branch name
        let head = self.repo.head()?;
        let branch_name = if head.is_branch() {
            head.shorthand().unwrap_or("master").to_string()
        } else {
            // Detached HEAD - push to master
            "master".to_string()
        };

        // Push current branch
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        remote.push(&[&refspec], None)
            .context("Failed to push to remote")?;

        log::info!("Git undo: pushed {} to remote", branch_name);
        Ok(())
    }

    /// Pull from remote to sync state
    ///
    /// Used on connect to get latest state from Nextcloud.
    pub fn pull(&self) -> Result<()> {
        let remote_path = self.remote_path.as_ref()
            .ok_or_else(|| anyhow!("No remote configured"))?;

        log::info!("Git undo: pulling from {}", remote_path.display());

        let mut remote = self.repo.find_remote("origin")
            .context("Failed to find origin remote")?;

        // Fetch all branches
        remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)
            .context("Failed to fetch from remote")?;

        // Get the remote master branch
        let fetch_head = match self.repo.find_reference("refs/remotes/origin/master") {
            Ok(r) => r,
            Err(_) => {
                log::info!("Git undo: no remote master branch yet, skipping pull");
                return Ok(());
            }
        };

        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        // Fast-forward merge if possible
        let head = self.repo.head()?;
        if let Some(head_oid) = head.target() {
            let head_commit = self.repo.find_commit(head_oid)?;
            let fetch_commit_obj = self.repo.find_commit(fetch_commit.id())?;

            // Check if we can fast-forward
            if self.repo.graph_descendant_of(fetch_commit.id(), head_oid)? {
                // Remote is ahead, fast-forward
                let refname = head.name().unwrap_or("refs/heads/master");
                self.repo.reference(
                    refname,
                    fetch_commit.id(),
                    true,
                    "Fast-forward pull",
                )?;

                // Checkout the new HEAD
                self.repo.checkout_tree(
                    fetch_commit_obj.tree()?.as_object(),
                    Some(git2::build::CheckoutBuilder::new().force()),
                )?;

                log::info!("Git undo: fast-forwarded to {}", fetch_commit.id());
            } else {
                log::info!("Git undo: local and remote have diverged, keeping local state");
            }
        }

        Ok(())
    }

    /// Get the local working directory path
    pub fn local_path(&self) -> &Path {
        &self.local_path
    }

    /// Get the working directory path
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Check if this repo has a remote configured
    pub fn has_remote(&self) -> bool {
        self.remote_path.is_some()
    }

    /// Open or initialize a local-only git repo (no remote)
    ///
    /// Used for tests and fallback when WebDAV mount is unavailable.
    pub fn open_or_init(local_path: &Path) -> Result<Self> {
        let repo = if local_path.join(".git").exists() {
            Repository::open(local_path)
                .context("Failed to open existing git repository")?
        } else {
            std::fs::create_dir_all(local_path)?;
            let repo = Repository::init(local_path)
                .context("Failed to initialize git repository")?;

            // Create initial commit so we have a valid HEAD
            {
                let sig = Signature::now("gradesta", "gradesta@local")?;
                let tree_id = {
                    let mut index = repo.index()?;
                    index.write_tree()?
                };
                let tree = repo.find_tree(tree_id)?;
                repo.commit(Some("HEAD"), &sig, &sig, "Initial state", &tree, &[])?;
            }

            log::info!("Initialized new local git repository");
            repo
        };

        Ok(GitUndoRepo {
            repo,
            local_path: local_path.to_path_buf(),
            remote_path: None,
        })
    }

    /// Stage all changes and create a commit
    /// Returns the new commit's Oid
    /// Note: Call sync_to_nextcloud() after this to persist the commit
    pub fn commit_all(&self, message: &str, author: &str) -> Result<Oid> {
        log::info!("commit_all: Starting commit for '{}' by '{}'", message, author);

        let mut index = self.repo.index()?;

        // Add all changes (including new files, modifications, and deletions)
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.update_all(["*"].iter(), None)?; // Handle deletions

        // Log what we're staging
        let statuses = self.repo.statuses(None)?;
        log::info!("commit_all: {} files to stage", statuses.len());
        for entry in statuses.iter() {
            log::debug!("  - {:?}: {}", entry.status(), entry.path().unwrap_or("?"));
        }

        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        // Get current HEAD as parent (if exists)
        let parent_commit = self.head_commit_obj();

        // Check if there are actual changes
        if let Some(ref parent) = parent_commit {
            if parent.tree_id() == tree_id {
                log::info!("commit_all: No changes to commit (tree unchanged)");
                return Ok(parent.id());
            }
        }

        let parents: Vec<&Commit> = parent_commit.iter().collect();

        // Create signature
        let sig = Signature::now(author, &format!("{}@gradesta", author))?;

        // Create commit
        let oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )?;

        log::info!("commit_all: Created commit {} for: {}", oid, message);
        Ok(oid)
    }

    /// Checkout a specific commit (for undo navigation)
    /// This updates the working tree to match the commit
    pub fn checkout_commit(&self, oid: Oid) -> Result<()> {
        let commit = self.repo.find_commit(oid)
            .context("Commit not found")?;

        // Check if we're at a branch tip - if so, we might need to create a new branch
        let was_at_branch_tip = self.is_at_branch_tip();

        // Checkout the commit
        let obj = commit.as_object();
        self.repo.checkout_tree(obj, Some(
            git2::build::CheckoutBuilder::new()
                .force() // Overwrite working tree
                .remove_untracked(true)
        ))?;

        // Move HEAD to the commit (detached HEAD state)
        self.repo.set_head_detached(oid)?;

        log::info!("Checked out commit {}", oid);

        // Note: If a change is made while in detached HEAD, ensure_on_branch
        // should be called to create a new branch before committing

        Ok(())
    }

    /// Get the current HEAD commit Oid (if any)
    pub fn head_commit(&self) -> Option<Oid> {
        self.repo.head().ok()
            .and_then(|r| r.target())
    }

    /// Get the current HEAD commit object (if any)
    fn head_commit_obj(&self) -> Option<Commit<'_>> {
        self.head_commit()
            .and_then(|oid| self.repo.find_commit(oid).ok())
    }

    /// Check if HEAD is at a branch tip
    fn is_at_branch_tip(&self) -> bool {
        let head = match self.repo.head() {
            Ok(h) => h,
            Err(_) => return false,
        };

        // If HEAD is not detached, it's at a branch tip
        !head.is_branch() == false
    }

    /// If in detached HEAD state, create a new branch at current position
    /// This is called before making a new commit while at an old state
    pub fn ensure_on_branch(&self) -> Result<String> {
        let head = self.repo.head()?;

        if !head.is_branch() {
            // We're in detached HEAD state, create a new branch
            let head_oid = head.target()
                .ok_or_else(|| anyhow!("HEAD has no target"))?;
            let commit = self.repo.find_commit(head_oid)?;

            // Generate branch name based on timestamp (milliseconds for uniqueness)
            let base_timestamp = chrono::Utc::now().timestamp_millis();
            let mut branch_name = format!("undo-branch-{}", base_timestamp);

            // If branch already exists, add a counter suffix
            let mut counter = 0;
            while self.repo.find_branch(&branch_name, BranchType::Local).is_ok() {
                counter += 1;
                branch_name = format!("undo-branch-{}-{}", base_timestamp, counter);
            }

            // Create the branch
            self.repo.branch(&branch_name, &commit, false)?;

            // Switch HEAD to the new branch
            self.repo.set_head(&format!("refs/heads/{}", branch_name))?;

            log::info!("Created new branch {} at {}", branch_name, head_oid);
            Ok(branch_name)
        } else {
            // Already on a branch
            Ok(head.shorthand().unwrap_or("main").to_string())
        }
    }

    /// Get all commits for undo tree visualization
    /// Returns commits in topological order (oldest first)
    pub fn get_all_commits(&self) -> Result<Vec<CommitInfo>> {
        log::info!("get_all_commits: Starting...");

        let mut commits = Vec::new();
        let mut seen = HashSet::new();

        // Walk all branches to get all commits
        let branches = self.repo.branches(Some(BranchType::Local))?;
        let mut branch_count = 0;

        for branch_result in branches {
            let (branch, _) = branch_result?;
            branch_count += 1;
            if let Some(oid) = branch.get().target() {
                log::debug!("get_all_commits: Walking branch {:?} at {}", branch.name(), oid);
                self.collect_commits(oid, &mut commits, &mut seen)?;
            }
        }

        log::info!("get_all_commits: Found {} local branches", branch_count);

        // Also include detached HEAD if present
        if let Some(head_oid) = self.head_commit() {
            log::debug!("get_all_commits: Also collecting from HEAD at {}", head_oid);
            self.collect_commits(head_oid, &mut commits, &mut seen)?;
        }

        // Sort by timestamp (oldest first for tree building)
        commits.sort_by_key(|c| c.timestamp);

        log::info!("get_all_commits: Returning {} commits", commits.len());
        Ok(commits)
    }

    /// Recursively collect commits from a starting point
    fn collect_commits(
        &self,
        oid: Oid,
        commits: &mut Vec<CommitInfo>,
        seen: &mut HashSet<Oid>,
    ) -> Result<()> {
        if seen.contains(&oid) {
            return Ok(());
        }
        seen.insert(oid);

        let commit = self.repo.find_commit(oid)?;

        let parent_oids: Vec<Oid> = commit.parent_ids().collect();

        // Recurse to parents first (to get oldest first)
        for parent_oid in &parent_oids {
            self.collect_commits(*parent_oid, commits, seen)?;
        }

        commits.push(CommitInfo {
            oid,
            message: commit.message().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("unknown").to_string(),
            timestamp: commit.time().seconds(),
            parent_oids,
        });

        Ok(())
    }

    /// Get commits that are children of a given commit
    /// (commits that have this commit as a parent)
    pub fn get_children(&self, parent_oid: Oid, all_commits: &[CommitInfo]) -> Vec<Oid> {
        all_commits.iter()
            .filter(|c| c.parent_oids.contains(&parent_oid))
            .map(|c| c.oid)
            .collect()
    }

    /// Check if the working tree has uncommitted changes
    pub fn has_changes(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        Ok(!statuses.is_empty())
    }

    /// Get the diff of changes that would be committed
    pub fn get_status_summary(&self) -> Result<String> {
        let statuses = self.repo.statuses(None)?;
        let mut summary = Vec::new();

        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry.path().unwrap_or("?");

            if status.is_index_new() || status.is_wt_new() {
                summary.push(format!("A {}", path));
            } else if status.is_index_modified() || status.is_wt_modified() {
                summary.push(format!("M {}", path));
            } else if status.is_index_deleted() || status.is_wt_deleted() {
                summary.push(format!("D {}", path));
            }
        }

        Ok(summary.join(", "))
    }
}

/// Convert commit Oid to vertex hash (for protocol)
pub fn commit_to_vertex_hash(oid: &Oid) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    // Add prefix to avoid collision with note vertex hashes
    "git-undo:".hash(&mut hasher);
    oid.as_bytes().hash(&mut hasher);
    hasher.finish()
}

/// Find commit Oid by vertex hash
pub fn vertex_hash_to_oid(commits: &[CommitInfo], hash: u64) -> Option<Oid> {
    commits.iter()
        .find(|c| commit_to_vertex_hash(&c.oid) == hash)
        .map(|c| c.oid)
}

/// Build edge structure for undo tree visualization
/// Returns: (west, east, north, south, up, down) vertex hashes
pub fn build_commit_edges(
    commit: &CommitInfo,
    all_commits: &[CommitInfo],
    children: &[Oid],
) -> [u64; 6] {
    let mut edges = [0u64; 6];

    // West: first parent (previous state)
    if let Some(parent_oid) = commit.parent_oids.first() {
        edges[0] = commit_to_vertex_hash(parent_oid);
    }

    // East: first child (next state on main branch)
    // If multiple children (branches), east goes to the first one, others are siblings
    if let Some(child_oid) = children.first() {
        edges[1] = commit_to_vertex_hash(child_oid);
    }

    // North/South: sibling commits (same parent, different branches)
    // Find ALL siblings (including self) and sort by timestamp for stable ordering
    if let Some(parent_oid) = commit.parent_oids.first() {
        let mut siblings: Vec<&CommitInfo> = all_commits.iter()
            .filter(|c| c.parent_oids.first() == Some(parent_oid))
            .collect();

        // Sort by timestamp for stable ordering
        siblings.sort_by_key(|c| c.timestamp);

        // Find our position in the sorted list
        if let Some(our_idx) = siblings.iter().position(|c| c.oid == commit.oid) {
            // North: previous sibling (if any)
            if our_idx > 0 {
                edges[2] = commit_to_vertex_hash(&siblings[our_idx - 1].oid);
            }
            // South: next sibling (if any)
            if our_idx + 1 < siblings.len() {
                edges[3] = commit_to_vertex_hash(&siblings[our_idx + 1].oid);
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_open_or_init_creates_repo() {
        let dir = tempdir().unwrap();
        let notes_dir = dir.path().join(".gradesta-notes");
        fs::create_dir_all(&notes_dir).unwrap();

        let repo = GitUndoRepo::open_or_init(&notes_dir).unwrap();

        // Should have initial commit
        assert!(repo.head_commit().is_some());

        // .git directory should exist
        assert!(notes_dir.join(".git").exists());
    }

    #[test]
    fn test_commit_all() {
        let dir = tempdir().unwrap();
        let notes_dir = dir.path().join(".gradesta-notes");
        fs::create_dir_all(&notes_dir).unwrap();

        let repo = GitUndoRepo::open_or_init(&notes_dir).unwrap();

        // Create a test file
        fs::write(notes_dir.join("test.txt"), "hello").unwrap();

        // Commit
        let oid = repo.commit_all("Test commit", "test-user").unwrap();

        // Should have new HEAD
        assert_eq!(repo.head_commit(), Some(oid));

        // Get all commits
        let commits = repo.get_all_commits().unwrap();
        assert_eq!(commits.len(), 2); // Initial + our commit
        assert!(commits.iter().any(|c| c.message == "Test commit"));
    }

    #[test]
    fn test_checkout_commit() {
        let dir = tempdir().unwrap();
        let notes_dir = dir.path().join(".gradesta-notes");
        fs::create_dir_all(&notes_dir).unwrap();

        let repo = GitUndoRepo::open_or_init(&notes_dir).unwrap();
        let initial_oid = repo.head_commit().unwrap();

        // Create and commit a file
        fs::write(notes_dir.join("test.txt"), "hello").unwrap();
        let _second_oid = repo.commit_all("Add test file", "test-user").unwrap();

        // Verify file exists
        assert!(notes_dir.join("test.txt").exists());

        // Checkout initial commit
        repo.checkout_commit(initial_oid).unwrap();

        // File should no longer exist
        assert!(!notes_dir.join("test.txt").exists());
    }

    #[test]
    fn test_commit_to_vertex_hash() {
        let oid1 = Oid::from_str("0000000000000000000000000000000000000001").unwrap();
        let oid2 = Oid::from_str("0000000000000000000000000000000000000002").unwrap();

        let hash1 = commit_to_vertex_hash(&oid1);
        let hash2 = commit_to_vertex_hash(&oid2);

        // Same oid should give same hash
        assert_eq!(hash1, commit_to_vertex_hash(&oid1));

        // Different oids should give different hashes
        assert_ne!(hash1, hash2);
    }
}
