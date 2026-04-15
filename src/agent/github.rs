//! GitHub PR creation and management
//!
//! Wraps the gh CLI to automatically create PRs.
//! Auto-extracts PR title/body from sprint release notes.
//!
//! Features:
//!   - PR creation (auto-sets title, body, branch)
//!   - Automatic PR creation on sprint release
//!   - PR listing
//!   - Auto-generated code review comments

use anyhow::{Context, Result};
use std::process::Command;

// ─── gh CLI wrapper ───────────────────────────────────────────────────────────

/// Check if gh CLI is installed
pub fn gh_available() -> bool {
    Command::new("gh").arg("--version").output().is_ok()
}

/// Get current branch name
pub fn current_branch() -> String {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "main".to_string())
        .trim()
        .to_string()
}

/// Default branch (main/master)
pub fn default_branch() -> String {
    let out = Command::new("git")
        .args(["remote", "show", "origin"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    if out.contains("HEAD branch: main") { "main".to_string() }
    else if out.contains("HEAD branch: master") { "master".to_string() }
    else { "main".to_string() }
}

// ─── PR options ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PrOptions {
    pub title: String,
    pub body: String,
    pub base: String,
    pub draft: bool,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
}

impl PrOptions {
    pub fn new(title: &str, body: &str) -> Self {
        Self {
            title: title.to_string(),
            body: body.to_string(),
            base: default_branch(),
            draft: false,
            labels: vec![],
            assignees: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn draft(mut self) -> Self { self.draft = true; self }
    #[allow(dead_code)]
    pub fn label(mut self, l: &str) -> Self { self.labels.push(l.to_string()); self }
    #[allow(dead_code)]
    pub fn base(mut self, b: &str) -> Self { self.base = b.to_string(); self }
}

// ─── PR creation ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PrResult {
    pub url: String,
    #[allow(dead_code)]
    pub number: u32,
}

/// Create PR (using gh CLI)
pub fn create_pr(opts: &PrOptions) -> Result<PrResult> {
    if !gh_available() {
        anyhow::bail!("gh CLI is not installed. Install it from https://cli.github.com");
    }

    let mut args = vec![
        "pr".to_string(), "create".to_string(),
        "--title".to_string(), opts.title.clone(),
        "--body".to_string(), opts.body.clone(),
        "--base".to_string(), opts.base.clone(),
    ];

    if opts.draft { args.push("--draft".to_string()); }
    for label in &opts.labels {
        args.extend(["--label".to_string(), label.clone()]);
    }
    for assignee in &opts.assignees {
        args.extend(["--assignee".to_string(), assignee.clone()]);
    }

    let output = Command::new("gh")
        .args(&args)
        .output()
        .context("Failed to run gh pr create")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to create PR: {}", stderr);
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let number = url.split('/').last()
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or(0);

    Ok(PrResult { url, number })
}

/// List PRs
pub fn list_prs(state: &str) -> Result<String> {
    if !gh_available() {
        return Ok("gh CLI is not installed".to_string());
    }
    let output = Command::new("gh")
        .args(["pr", "list", "--state", state, "--limit", "20"])
        .output()
        .context("Failed to run gh pr list")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Add a comment to a PR
#[allow(dead_code)]
pub fn add_pr_comment(pr_number: u32, comment: &str) -> Result<()> {
    if !gh_available() {
        anyhow::bail!("gh CLI is not installed");
    }
    let output = Command::new("gh")
        .args(["pr", "comment", &pr_number.to_string(), "--body", comment])
        .output()
        .context("Failed to run gh pr comment")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to add comment: {}", stderr);
    }
    Ok(())
}

// ─── Auto-create sprint release PR ──────────────────────────────────────────

/// Auto-generate PR body from release notes
#[allow(dead_code)]
pub fn build_pr_body_from_release_notes(
    sprint_id: &str,
    story_titles: &[&str],
    release_notes: &str,
    bugs_fixed: usize,
    security_findings: usize,
) -> String {
    let story_list = story_titles.iter()
        .map(|t| format!("- {}", t))
        .collect::<Vec<_>>().join("\n");

    format!(
        "## Sprint {} Release\n\n\
         ### Implemented features\n{}\n\n\
         ### Stats\n\
         - Bugs fixed: {}\n\
         - Security findings resolved: {}\n\n\
         ### Release notes\n{}\n\n\
         ---\n\
         *This PR was auto-generated by the AI agent pipeline.*",
        sprint_id, story_list, bugs_fixed, security_findings,
        crate::utils::trunc(release_notes, 1000)
    )
}

/// Auto-create PR after sprint completion (optional)
#[allow(dead_code)]
pub fn auto_create_sprint_pr(
    sprint_id: &str,
    story_titles: &[&str],
    release_notes: &str,
    bugs_fixed: usize,
    security_findings: usize,
) -> Result<PrResult> {
    let branch = current_branch();
    let base = default_branch();

    if branch == base {
        anyhow::bail!("Current branch ({}) is the same as the base branch. Run this from a separate branch.", branch);
    }

    let title = format!("feat: {} sprint release — {} feature(s)", sprint_id, story_titles.len());
    let body = build_pr_body_from_release_notes(
        sprint_id, story_titles, release_notes, bugs_fixed, security_findings
    );

    let opts = PrOptions::new(&title, &body)
        .label("sprint-release")
        .label("automated");

    create_pr(&opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_pr_body_contains_sprint_id() {
        let body = build_pr_body_from_release_notes(
            "S3", &["login feature", "sign-up"], "Version 1.0.0 release", 3, 1
        );
        assert!(body.contains("S3"));
        assert!(body.contains("login feature"));
        assert!(body.contains("Bugs fixed: 3"));
        assert!(body.contains("Security findings resolved: 1"));
    }

    #[test]
    fn test_pr_options_draft() {
        let opts = PrOptions::new("title", "body").draft().label("bug");
        assert!(opts.draft);
        assert_eq!(opts.labels, vec!["bug"]);
    }

    #[test]
    fn test_pr_options_base() {
        let opts = PrOptions::new("title", "body").base("develop");
        assert_eq!(opts.base, "develop");
    }
}
