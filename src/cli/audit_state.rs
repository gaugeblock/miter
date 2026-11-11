//! A commit deleting tracked `.gaugeblock/` state must cite a decision record (`MTR-1`).
//!
//! On 2026-11-06 ten repositories each committed "retire the legacy repo-local ledger", deleting their
//! `.gaugeblock/storypole/ledger.jsonl`. It read as routine hygiene. Nobody asked why the files had
//! frozen, and the answer - because nothing wrote to them, because write routing was never wired - was
//! the actual defect. Days went into rediscovering it. Requiring a decision record forces the sentence
//! that exposes the circularity: an ADR has to say why the state is superseded, and "because nothing
//! writes to it" is a statement that invites a second look.
//!
//! **The baseline is a revision, not an allowlist.** A full-history scan would need a hand-written
//! justification for each of those ten deletions and would leave an allowlist as the place to hide the
//! eleventh. Commits reachable from the baseline have been reviewed by whoever merged them; commits
//! since it have not, and those are the ones this gate reads. On a clean mainline it reports nothing,
//! which is the correct answer for history that is already merged.
//!
//! Known limit, stated rather than implied: a deletion committed straight to the baseline branch is not
//! seen. That is the branch discipline `AGENTS.md` Law 10 asks for, and the rev this gate is given is
//! the marker for where review begins.

use std::path::Path;
use std::process::Command;

use super::audit::AuditFormat;
use super::error::CliError;

/// One commit that deletes tracked suite state without citing a decision record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StateDeletion {
    /// Short commit id, enough to find it.
    pub(super) commit: String,
    /// The commit subject, so the report reads as a sentence.
    pub(super) subject: String,
    /// Tracked files the commit deleted.
    pub(super) files: Vec<String>,
}

/// The revision this repository compares against: `origin/main`, else `main`, else none.
///
/// `origin/main` first because the remote is the closest thing to reviewed history: a local `main` can be
/// ahead of it, and reading that range would flag deletions nobody has pushed yet - which is to say,
/// exactly the ones this gate exists to catch.
pub(super) fn baseline(root: &Path) -> Option<String> {
    ["origin/main", "main"]
        .into_iter()
        .find(|candidate| {
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["rev-parse", "--verify", "--quiet", candidate])
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false)
        })
        .map(str::to_string)
}

/// Commits since `baseline` that delete tracked paths under `.gaugeblock/`, without a citation.
///
/// # Errors
///
/// [`std::io::Error`] only when `git` cannot be spawned; a range git rejects is reported as no findings,
/// because a shallow or unusual checkout must not fail every audit in the suite.
pub(super) fn deletions_since(root: &Path, baseline: &str) -> std::io::Result<Vec<StateDeletion>> {
    let range = format!("{baseline}..HEAD");
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "log",
            "--diff-filter=D",
            "--name-only",
            "--pretty=format:%x1e%H",
            &range,
            "--",
            ".gaugeblock",
        ])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut deletions = Vec::new();
    for record in text
        .split('\u{1e}')
        .filter(|chunk| !chunk.trim().is_empty())
    {
        let mut lines = record.lines().filter(|line| !line.trim().is_empty());
        let Some(sha) = lines.next().map(str::trim) else {
            continue;
        };
        let files: Vec<String> = lines.map(|line| line.trim().to_string()).collect();
        if files.is_empty() {
            continue;
        }
        let (subject, message) = commit_text(root, sha);
        if cites_decision(&message) {
            continue;
        }
        deletions.push(StateDeletion {
            commit: sha.chars().take(12).collect(),
            subject,
            files,
        });
    }
    Ok(deletions)
}

/// The subject and full message of one commit, as `(subject, everything)`.
fn commit_text(root: &Path, sha: &str) -> (String, String) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", "-s", "--format=%s%n%b", sha])
        .output();
    let Ok(output) = output else {
        return (String::new(), String::new());
    };
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let subject = text.lines().next().unwrap_or_default().trim().to_string();
    (subject, text)
}

/// Whether a commit message cites a decision record.
///
/// Recognises `ADR-0007` and a path under `docs/decisions/` - the two spellings the suite uses.
/// Deliberately textual: the question is "did the author name the record", not "is the record well
/// formed", which is a different question with its own gate.
pub(super) fn cites_decision(text: &str) -> bool {
    let lower = text.to_lowercase();
    let after = |needle: &str| -> bool {
        let mut rest = lower.as_str();
        while let Some(index) = rest.find(needle) {
            rest = &rest[index + needle.len()..];
            if rest.starts_with(|c: char| c.is_ascii_digit()) {
                return true;
            }
        }
        false
    };
    after("adr-") || after("adr ") || after("decisions/")
}

/// Runs the check and reports it.
///
/// # Errors
///
/// [`CliError::Rejected`] when a commit since the baseline deleted tracked state without citing a record.
pub(super) fn run_state_audit(root: &Path, format: AuditFormat) -> Result<(), CliError> {
    let Some(baseline) = baseline(root) else {
        // No git, or no mainline to compare against: nothing is reviewable here, and inventing a
        // finding would make the gate fire in scratch checkouts that are not repositories at all.
        return Ok(());
    };
    let deletions = deletions_since(root, &baseline).map_err(CliError::Io)?;
    if deletions.is_empty() {
        return Ok(());
    }

    match format {
        AuditFormat::Json => {
            let rows: Vec<super::json::Json> = deletions
                .iter()
                .map(|deletion| {
                    super::json::Json::object(vec![
                        ("commit", super::json::Json::string(&deletion.commit)),
                        ("subject", super::json::Json::string(&deletion.subject)),
                        (
                            "files",
                            super::json::Json::array(
                                deletion
                                    .files
                                    .iter()
                                    .map(super::json::Json::string)
                                    .collect::<Vec<_>>(),
                            ),
                        ),
                    ])
                })
                .collect();
            println!(
                "{}",
                super::json::Json::object(vec![(
                    "state_deletions",
                    super::json::Json::array(rows)
                )])
                .render()
            );
        }
        AuditFormat::Table => {
            println!(
                "Commits since {baseline} delete tracked .gaugeblock state without citing a decision:"
            );
            for deletion in &deletions {
                println!("  {} {}", deletion.commit, deletion.subject);
                for file in &deletion.files {
                    println!("      {file}");
                }
            }
            println!(
                "Name the record in the commit message (ADR-NNNN or docs/decisions/NNNN-...), \
                 because the reason state is superseded is the finding."
            );
        }
    }

    Err(CliError::Rejected(format!(
        "{} commit(s) deleted tracked .gaugeblock state without citing a decision record",
        deletions.len()
    )))
}

#[cfg(test)]
mod tests;
