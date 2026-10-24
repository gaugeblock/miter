//! Git repository metadata extractor.

use std::fs;
use std::path::Path;
use std::process::Command;

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

fn get_current_branch(root_dir: &Path) -> Option<String> {
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root_dir)
        .output()
        && output.status.success()
    {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Some(branch);
        }
    }

    let head_file = root_dir.join(".git").join("HEAD");
    if let Ok(content) = fs::read_to_string(head_file)
        && let Some(rest) = content.trim().strip_prefix("ref: refs/heads/")
    {
        return Some(rest.to_string());
    }
    None
}

fn get_head_commit(root_dir: &Path) -> Option<String> {
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root_dir)
        .output()
        && output.status.success()
    {
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !hash.is_empty() {
            return Some(hash);
        }
    }
    None
}

fn get_latest_tag(root_dir: &Path) -> Option<String> {
    if let Ok(output) = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(root_dir)
        .output()
        && output.status.success()
    {
        let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !tag.is_empty() {
            return Some(tag);
        }
    }
    None
}

fn get_unreleased_commits(root_dir: &Path, latest_tag: Option<&str>) -> Vec<String> {
    let rev_range = if let Some(tag) = latest_tag {
        format!("{tag}..HEAD")
    } else {
        "HEAD~20..HEAD".to_string()
    };

    if let Ok(output) = Command::new("git")
        .args(["log", &rev_range, "--oneline"])
        .current_dir(root_dir)
        .output()
        && output.status.success()
    {
        let log = String::from_utf8_lossy(&output.stdout);
        return log
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }
    Vec::new()
}
