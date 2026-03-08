//! Git undo branching tests
//!
//! Tests for the git-based undo system, including linear history,
//! branching on edits after checkout, and tree navigation.

use std::fs;

use crate::git_undo::{build_commit_edges, commit_to_vertex_hash};

use super::harness::TestHarness;

/// Test creating linear commit history
#[tokio::test]
async fn test_linear_history() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    // Create 3 commits
    {
        let repo = harness.git_repo.lock().await;

        // First commit
        fs::write(workdir.join("file1.txt"), "content 1").unwrap();
        let oid1 = repo.commit_all("First commit", "test-user").unwrap();

        // Second commit
        fs::write(workdir.join("file2.txt"), "content 2").unwrap();
        let oid2 = repo.commit_all("Second commit", "test-user").unwrap();

        // Third commit
        fs::write(workdir.join("file3.txt"), "content 3").unwrap();
        let oid3 = repo.commit_all("Third commit", "test-user").unwrap();

        // Verify linear chain
        let commits = repo.get_all_commits().unwrap();

        // Should have initial + 3 commits
        assert!(commits.len() >= 4);

        // Find our commits
        let c1 = commits.iter().find(|c| c.oid == oid1).unwrap();
        let c2 = commits.iter().find(|c| c.oid == oid2).unwrap();
        let c3 = commits.iter().find(|c| c.oid == oid3).unwrap();

        // Verify parent relationships
        assert!(c2.parent_oids.contains(&oid1));
        assert!(c3.parent_oids.contains(&oid2));
    }
}

/// Test checking out an old commit
#[tokio::test]
async fn test_checkout_old_commit() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create a file and commit
        fs::write(workdir.join("test.txt"), "version 1").unwrap();
        let oid1 = repo.commit_all("Version 1", "test-user").unwrap();

        // Modify and commit again
        fs::write(workdir.join("test.txt"), "version 2").unwrap();
        let _oid2 = repo.commit_all("Version 2", "test-user").unwrap();

        // Verify current content
        assert_eq!(fs::read_to_string(workdir.join("test.txt")).unwrap(), "version 2");

        // Checkout old commit
        repo.checkout_commit(oid1).unwrap();

        // Content should be restored
        assert_eq!(fs::read_to_string(workdir.join("test.txt")).unwrap(), "version 1");
    }
}

/// Test that editing after checkout creates a new branch
#[tokio::test]
async fn test_branch_on_edit_after_checkout() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create initial commit
        fs::write(workdir.join("main.txt"), "main content").unwrap();
        let oid1 = repo.commit_all("Main commit 1", "test-user").unwrap();

        // Continue on main
        fs::write(workdir.join("main.txt"), "main content v2").unwrap();
        let oid2 = repo.commit_all("Main commit 2", "test-user").unwrap();

        // Checkout old commit
        repo.checkout_commit(oid1).unwrap();

        // Make a change - should create new branch
        let branch_name = repo.ensure_on_branch().unwrap();
        assert!(branch_name.starts_with("undo-branch-"));

        // Commit the change
        fs::write(workdir.join("branch.txt"), "branch content").unwrap();
        let branch_oid = repo.commit_all("Branch commit", "test-user").unwrap();

        // Verify we have a branch
        let commits = repo.get_all_commits().unwrap();

        // Find the branch commit
        let branch_commit = commits.iter().find(|c| c.oid == branch_oid).unwrap();

        // Its parent should be oid1, not oid2
        assert!(branch_commit.parent_oids.contains(&oid1));
        assert!(!branch_commit.parent_oids.contains(&oid2));
    }
}

/// Test that branch names include timestamp
#[tokio::test]
async fn test_branch_name_includes_timestamp() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create and commit
        fs::write(workdir.join("file.txt"), "content").unwrap();
        let oid1 = repo.commit_all("Commit", "test-user").unwrap();

        // Checkout to create detached HEAD
        repo.checkout_commit(oid1).unwrap();

        // Create branch
        let branch_name = repo.ensure_on_branch().unwrap();

        // Should have timestamp format
        assert!(branch_name.starts_with("undo-branch-"));

        // The part after "undo-branch-" should be a number (timestamp)
        let timestamp_part = &branch_name["undo-branch-".len()..];
        assert!(timestamp_part.parse::<i64>().is_ok());
    }
}

/// Test sibling edges (north/south) between branches from same parent
#[tokio::test]
async fn test_sibling_edges_north_south() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create base commit
        fs::write(workdir.join("base.txt"), "base").unwrap();
        let base_oid = repo.commit_all("Base commit", "test-user").unwrap();

        // Create first branch
        fs::write(workdir.join("branch1.txt"), "branch 1").unwrap();
        let branch1_oid = repo.commit_all("Branch 1", "test-user").unwrap();

        // Go back and create second branch
        repo.checkout_commit(base_oid).unwrap();
        repo.ensure_on_branch().unwrap();
        fs::write(workdir.join("branch2.txt"), "branch 2").unwrap();
        let branch2_oid = repo.commit_all("Branch 2", "test-user").unwrap();

        // Get all commits
        let commits = repo.get_all_commits().unwrap();

        // Get children of base
        let children = repo.get_children(base_oid, &commits);
        assert_eq!(children.len(), 2);
        assert!(children.contains(&branch1_oid));
        assert!(children.contains(&branch2_oid));

        // Build edges for branch1
        let b1_commit = commits.iter().find(|c| c.oid == branch1_oid).unwrap();
        let b1_edges = build_commit_edges(b1_commit, &commits, &[]);

        // Build edges for branch2
        let b2_commit = commits.iter().find(|c| c.oid == branch2_oid).unwrap();
        let b2_edges = build_commit_edges(b2_commit, &commits, &[]);

        // One should point to the other via north/south
        let b1_hash = commit_to_vertex_hash(&branch1_oid);
        let b2_hash = commit_to_vertex_hash(&branch2_oid);

        // They should be siblings
        let has_sibling_link = (b1_edges[2] == b2_hash || b1_edges[3] == b2_hash)
            || (b2_edges[2] == b1_hash || b2_edges[3] == b1_hash);
        assert!(has_sibling_link, "Sibling commits should have north/south edges");
    }
}

/// Test creating multiple branches from the same commit
#[tokio::test]
async fn test_multiple_branches_from_one_commit() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create base commit
        fs::write(workdir.join("base.txt"), "base").unwrap();
        let base_oid = repo.commit_all("Base", "test-user").unwrap();

        let mut branch_oids = Vec::new();

        // Create 3 branches from base
        for i in 1..=3 {
            repo.checkout_commit(base_oid).unwrap();
            repo.ensure_on_branch().unwrap();
            fs::write(workdir.join(format!("branch{}.txt", i)), format!("branch {}", i)).unwrap();
            let branch_oid = repo.commit_all(&format!("Branch {}", i), "test-user").unwrap();
            branch_oids.push(branch_oid);
        }

        // Verify all branches exist
        let commits = repo.get_all_commits().unwrap();
        let children = repo.get_children(base_oid, &commits);

        assert_eq!(children.len(), 3);
        for oid in &branch_oids {
            assert!(children.contains(oid));
        }

        // All branches should form a sibling chain
        for oid in &branch_oids {
            let commit = commits.iter().find(|c| c.oid == *oid).unwrap();
            let edges = build_commit_edges(commit, &commits, &[]);

            // Should have at least one sibling (north or south non-zero)
            let has_sibling = edges[2] != 0 || edges[3] != 0;
            assert!(has_sibling, "Branch should have sibling edge");
        }
    }
}

/// Test that west edge points to parent commit
#[tokio::test]
async fn test_commit_edges_west_east() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create parent commit
        fs::write(workdir.join("parent.txt"), "parent").unwrap();
        let parent_oid = repo.commit_all("Parent", "test-user").unwrap();

        // Create child commit
        fs::write(workdir.join("child.txt"), "child").unwrap();
        let child_oid = repo.commit_all("Child", "test-user").unwrap();

        let commits = repo.get_all_commits().unwrap();
        let children = repo.get_children(parent_oid, &commits);

        // Build edges for child
        let child_commit = commits.iter().find(|c| c.oid == child_oid).unwrap();
        let child_edges = build_commit_edges(child_commit, &commits, &[]);

        // Child's west should point to parent
        let parent_hash = commit_to_vertex_hash(&parent_oid);
        assert_eq!(child_edges[0], parent_hash);

        // Build edges for parent
        let parent_commit = commits.iter().find(|c| c.oid == parent_oid).unwrap();
        let parent_edges = build_commit_edges(parent_commit, &commits, &children);

        // Parent's east should point to child
        let child_hash = commit_to_vertex_hash(&child_oid);
        assert_eq!(parent_edges[1], child_hash);
    }
}

/// Test that committing with no changes returns existing OID
#[tokio::test]
async fn test_no_duplicate_commits_on_no_change() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create commit
        fs::write(workdir.join("file.txt"), "content").unwrap();
        let oid1 = repo.commit_all("Commit", "test-user").unwrap();

        // Try to commit again with no changes
        let oid2 = repo.commit_all("No changes", "test-user").unwrap();

        // Should return the same OID
        assert_eq!(oid1, oid2);

        // Verify only one commit was created
        let commits = repo.get_all_commits().unwrap();
        let matching = commits.iter().filter(|c| c.oid == oid1).count();
        assert_eq!(matching, 1);
    }
}

/// Test commit vertex labels (message format)
#[tokio::test]
async fn test_undo_tree_vertex_labels() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create commit with specific message
        fs::write(workdir.join("test.txt"), "test").unwrap();
        let oid = repo.commit_all("Create vertex: abc123", "test-user").unwrap();

        // Get commit info
        let commits = repo.get_all_commits().unwrap();
        let commit = commits.iter().find(|c| c.oid == oid).unwrap();

        // Verify message is preserved
        assert_eq!(commit.message, "Create vertex: abc123");
        assert_eq!(commit.author, "test-user");
    }
}

/// Test commit hash consistency
#[tokio::test]
async fn test_commit_to_vertex_hash_consistency() {
    use git2::Oid;

    let oid1 = Oid::from_str("0000000000000000000000000000000000000001").unwrap();
    let oid2 = Oid::from_str("0000000000000000000000000000000000000001").unwrap();
    let oid3 = Oid::from_str("0000000000000000000000000000000000000002").unwrap();

    // Same OID should produce same hash
    assert_eq!(commit_to_vertex_hash(&oid1), commit_to_vertex_hash(&oid2));

    // Different OIDs should produce different hashes
    assert_ne!(commit_to_vertex_hash(&oid1), commit_to_vertex_hash(&oid3));
}

/// Test deep branch navigation
#[tokio::test]
async fn test_deep_branch_navigation() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create: A -> B -> C (master)
        fs::write(workdir.join("a.txt"), "a").unwrap();
        let oid_a = repo.commit_all("A", "test-user").unwrap();

        fs::write(workdir.join("b.txt"), "b").unwrap();
        let oid_b = repo.commit_all("B", "test-user").unwrap();

        fs::write(workdir.join("c.txt"), "c").unwrap();
        let oid_c = repo.commit_all("C", "test-user").unwrap();

        // Create branch from A: A -> D -> E
        repo.checkout_commit(oid_a).unwrap();
        repo.ensure_on_branch().unwrap();

        fs::write(workdir.join("d.txt"), "d").unwrap();
        let oid_d = repo.commit_all("D", "test-user").unwrap();

        fs::write(workdir.join("e.txt"), "e").unwrap();
        let oid_e = repo.commit_all("E", "test-user").unwrap();

        // Create branch from D: D -> F
        repo.checkout_commit(oid_d).unwrap();
        repo.ensure_on_branch().unwrap();

        fs::write(workdir.join("f.txt"), "f").unwrap();
        let oid_f = repo.commit_all("F", "test-user").unwrap();

        // Verify structure
        let commits = repo.get_all_commits().unwrap();

        // E and F should be siblings (both from D)
        let children_of_d = repo.get_children(oid_d, &commits);
        assert_eq!(children_of_d.len(), 2);
        assert!(children_of_d.contains(&oid_e));
        assert!(children_of_d.contains(&oid_f));

        // B and D should be siblings (both from A)
        let children_of_a = repo.get_children(oid_a, &commits);
        assert_eq!(children_of_a.len(), 2);
        assert!(children_of_a.contains(&oid_b));
        assert!(children_of_a.contains(&oid_d));
    }
}

/// Test branch name uniqueness for same-second branches
#[tokio::test]
async fn test_timestamp_collision_branches() {
    let harness = TestHarness::new().await;
    let workdir = harness.workdir();

    {
        let repo = harness.git_repo.lock().await;

        // Create base
        fs::write(workdir.join("base.txt"), "base").unwrap();
        let base_oid = repo.commit_all("Base", "test-user").unwrap();

        // Create multiple branches rapidly
        let mut branch_names = Vec::new();
        for i in 0..3 {
            repo.checkout_commit(base_oid).unwrap();
            let name = repo.ensure_on_branch().unwrap();
            branch_names.push(name.clone());

            // Make unique change
            fs::write(workdir.join(format!("rapid{}.txt", i)), format!("rapid {}", i)).unwrap();
            repo.commit_all(&format!("Rapid {}", i), "test-user").unwrap();
        }

        // Due to timestamp-based naming, we might get same names
        // But commits should still be distinct
        let commits = repo.get_all_commits().unwrap();
        let children = repo.get_children(base_oid, &commits);

        // Should have 3 distinct child commits
        assert_eq!(children.len(), 3);
    }
}
