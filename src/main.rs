use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(name = "gh-pr-sync")]
#[command(about = "Sync GitHub PRs to local YAML files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Pull {
        #[arg(short, long)]
        repo: Option<String>,
        /// Maximum number of PRs to fetch
        #[arg(short, long, default_value = "100")]
        limit: u32,
        /// Include closed PRs
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPullRequest {
    number: u64,
    title: String,
    state: String,
    author: Option<GhAuthor>,
    head_ref_name: String,
    base_ref_name: String,
    labels: Option<GhLabels>,
    files: Option<GhFiles>,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
    body: Option<String>,
    additions: u64,
    deletions: u64,
    #[serde(default)]
    is_draft: bool,
}

#[derive(Debug, Deserialize)]
struct GhAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GhLabels {
    nodes: Vec<GhLabel>,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GhFiles {
    nodes: Vec<GhFile>,
}

#[derive(Debug, Deserialize)]
struct GhFile {
    path: String,
    additions: u64,
    deletions: u64,
}

#[derive(Debug, Serialize)]
struct PullRequest {
    number: u64,
    title: String,
    state: String,
    author: String,
    head: String,
    base: String,
    labels: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merged_at: Option<DateTime<Utc>>,
    additions: u64,
    deletions: u64,
    is_draft: bool,
    files: Vec<FileChange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileChange {
    path: String,
    additions: u64,
    deletions: u64,
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn pull_prs(repo: Option<String>, limit: u32, all: bool) -> Result<()> {
    let state_filter = if all { "all" } else { "open" };

    let mut args = vec![
        "pr".to_string(),
        "list".to_string(),
        "--state".to_string(),
        state_filter.to_string(),
        "--limit".to_string(),
        limit.to_string(),
        "--json".to_string(),
        "number,title,state,author,headRefName,baseRefName,labels,files,createdAt,updatedAt,mergedAt,body,additions,deletions,isDraft".to_string(),
    ];

    if let Some(ref r) = repo {
        args.push("--repo".to_string());
        args.push(r.clone());
    }

    eprintln!("Fetching PRs...");

    let output = Command::new("gh")
        .args(&args)
        .output()
        .context("Failed to run gh cli. Is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr list failed: {}", stderr);
    }

    let prs: Vec<GhPullRequest> =
        serde_json::from_slice(&output.stdout).context("Failed to parse gh output")?;

    eprintln!("Found {} PRs", prs.len());

    let prs_dir = Path::new(".prs");
    fs::create_dir_all(prs_dir).context("Failed to create .prs directory")?;

    if prs_dir.exists() {
        for entry in fs::read_dir(prs_dir)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .map(|e| e == "yaml")
                .unwrap_or(false)
            {
                fs::remove_file(entry.path())?;
            }
        }
    }

    for gh_pr in prs {
        let pr = PullRequest {
            number: gh_pr.number,
            title: gh_pr.title.clone(),
            state: gh_pr.state.to_lowercase(),
            author: gh_pr.author.map(|a| a.login).unwrap_or_default(),
            head: gh_pr.head_ref_name,
            base: gh_pr.base_ref_name,
            labels: gh_pr
                .labels
                .map(|l| l.nodes.into_iter().map(|n| n.name).collect())
                .unwrap_or_default(),
            created_at: gh_pr.created_at.parse().context("Invalid created_at")?,
            updated_at: gh_pr.updated_at.parse().context("Invalid updated_at")?,
            merged_at: gh_pr
                .merged_at
                .as_ref()
                .map(|s| s.parse())
                .transpose()
                .context("Invalid merged_at")?,
            additions: gh_pr.additions,
            deletions: gh_pr.deletions,
            is_draft: gh_pr.is_draft,
            files: gh_pr
                .files
                .map(|f| {
                    f.nodes
                        .into_iter()
                        .map(|n| FileChange {
                            path: n.path,
                            additions: n.additions,
                            deletions: n.deletions,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            body: gh_pr.body.filter(|b| !b.is_empty()),
        };

        let slug = slugify(&pr.title);
        let filename = format!("{}-{}.yaml", pr.number, slug);
        let path = prs_dir.join(&filename);

        let yaml = serde_yaml::to_string(&pr).context("Failed to serialize PR")?;
        fs::write(&path, yaml).context("Failed to write PR file")?;

        eprintln!("  {} {}", pr.number, pr.title);
    }

    eprintln!("Synced to .prs/");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pull { repo, limit, all } => pull_prs(repo, limit, all),
    }
}
