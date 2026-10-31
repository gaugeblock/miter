//! Git repository metadata extractor.

use std::path::Path;

use crate::inventory::types::GitInventory;

/// Extracts Git metadata for the workspace root.
pub fn extract_git_inventory(root_dir: &Path) -> Option<GitInventory> {
    if !root_dir.join(".git").exists() {
        return None;
    }

    let current_branch = get_current_branch(root_dir).unwrap_or_else(|| "HEAD".to_string());
    let head_commit = get_head_commit(root_dir).unwrap_or_else(|| "unknown".to_string());
    let latest_tag = get_latest_tag(root_dir);
    let unreleased_commits = get_unreleased_commits(root_dir, latest_tag.as_deref());

    Some(GitInventory {
        current_branch,
        head_commit,
        latest_tag,
        unreleased_commits,
    })
}

/// Opens the repository, or `None` when the directory is not one.
fn open(root_dir: &Path) -> Option<gaugeblock_git::Repository> {
    gaugeblock_git::Repository::discover(root_dir).ok()
}

fn get_current_branch(root_dir: &Path) -> Option<String> {
    // The crate reports an unborn HEAD's branch too, which is what the old `.git/HEAD` fallback was
    // reconstructing by hand.
    open(root_dir)?.current_branch().ok().flatten()
}

fn get_head_commit(root_dir: &Path) -> Option<String> {
    open(root_dir)?.head_sha().ok()
}

fn get_latest_tag(root_dir: &Path) -> Option<String> {
    // No tags is the normal state for a young repository, so git's refusal becomes `None`.
    open(root_dir)?.describe_tags().ok()
}

fn get_unreleased_commits(root_dir: &Path, latest_tag: Option<&str>) -> Vec<String> {
    let rev_range = match latest_tag {
        Some(tag) => format!("{tag}..HEAD"),
        None => "HEAD~20..HEAD".to_string(),
    };
    let Some(repo) = open(root_dir) else {
        return Vec::new();
    };
    // `--oneline` was sha-then-summary; the crate's Commit is the same pair, so the shape survives.
    match repo.commits_between(&rev_range) {
        Ok(commits) => commits
            .into_iter()
            .map(|commit| {
                format!(
                    "{} {}",
                    &commit.sha[..7.min(commit.sha.len())],
                    commit.summary
                )
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}
