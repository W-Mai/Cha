use std::process::Command;
use std::time::{Duration, Instant};

use crate::{Result, cmd_publish, project_root, read_workspace_version, run_cmd};

fn gh(args: &[&str]) -> Result<String> {
    let out = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("gh {}: {e}", args.join(" ")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!(
            "gh {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into())
    }
}

/// Filter a `gh run list` query to the run that matches the commit we just
/// pushed, so we don't mistake an older run for the one we're waiting on.
enum RunFilter<'a> {
    /// Match by `headSha` — use for ci.yml on main.
    Sha(&'a str),
    /// Match by `headBranch` — use for release.yml which runs on tags.
    Branch(&'a str),
}

impl RunFilter<'_> {
    fn jq_select(&self) -> String {
        match self {
            RunFilter::Sha(s) => format!("select(.headSha == \"{s}\")"),
            RunFilter::Branch(b) => format!("select(.headBranch == \"{b}\")"),
        }
    }
    fn label(&self) -> String {
        match self {
            RunFilter::Sha(s) => format!("sha {}", &s[..7.min(s.len())]),
            RunFilter::Branch(b) => format!("branch {b}"),
        }
    }
}

enum PollState {
    NotYetQueued,
    InProgress { status: String, conclusion: String },
    Terminal { conclusion: String },
}

fn poll_workflow(workflow: &str, filter: &RunFilter) -> Result<PollState> {
    let out = gh(&[
        "run",
        "list",
        "--workflow",
        workflow,
        "--limit",
        "10",
        "--json",
        "status,conclusion,headSha,headBranch,databaseId",
        "-q",
        &format!(
            ".[] | {} | [.status, .conclusion] | @tsv",
            filter.jq_select()
        ),
    ])?;
    let Some(first) = out.lines().next().filter(|l| !l.is_empty()) else {
        return Ok(PollState::NotYetQueued);
    };
    let parts: Vec<&str> = first.split('\t').collect();
    let status = parts.first().copied().unwrap_or("").to_string();
    let conclusion = parts.get(1).copied().unwrap_or("").to_string();
    if status == "completed" {
        Ok(PollState::Terminal { conclusion })
    } else {
        Ok(PollState::InProgress { status, conclusion })
    }
}

/// Poll until the run matching `filter` reaches a terminal state.
/// Tolerates the run not appearing yet (GitHub takes a few seconds after push).
fn wait_for_workflow(workflow: &str, filter: RunFilter, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    println!("  → waiting for {workflow} ({}) ...", filter.label());
    let mut last_print = String::new();
    loop {
        std::thread::sleep(Duration::from_secs(15));
        match poll_workflow(workflow, &filter)? {
            PollState::NotYetQueued => {
                if last_print != "pending" {
                    println!("    {workflow}: (not yet queued)");
                    last_print = "pending".into();
                }
            }
            PollState::InProgress { status, conclusion } => {
                let msg = format!("{status}/{conclusion}");
                if msg != last_print {
                    println!("    {workflow}: {status} / {conclusion}");
                    last_print = msg;
                }
            }
            PollState::Terminal { conclusion } => {
                println!("    {workflow}: completed / {conclusion}");
                return if conclusion == "success" {
                    Ok(())
                } else {
                    Err(format!("{workflow} completed with: {conclusion}").into())
                };
            }
        }
        if start.elapsed() > timeout {
            return Err(format!("timeout waiting for {workflow}").into());
        }
    }
}

fn ensure_clean_tree(root: &str) -> Result {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()?;
    if !out.stdout.is_empty() {
        return Err("working tree is not clean — commit all changes first".into());
    }
    Ok(())
}

fn tag_and_push(root: &str, tag: &str) -> Result {
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let tag_commit = Command::new("git")
        .args(["rev-list", "-n1", tag])
        .current_dir(root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if !tag_commit.is_empty() && tag_commit == head {
        println!("  → tag {tag} already points to HEAD, skipping");
        return Ok(());
    }

    if !tag_commit.is_empty() {
        println!("  → tag {tag} points to old commit, re-tagging...");
        let _ = run_cmd("git", &["tag", "-d", tag]);
        let _ = run_cmd("git", &["push", "origin", &format!(":refs/tags/{tag}")]);
    }

    run_cmd("git", &["tag", tag])?;
    run_cmd("git", &["push", "origin", tag])?;
    Ok(())
}

fn wait_for_release_workflow(tag: &str) -> Result {
    let done = gh(&[
        "run",
        "list",
        "--workflow",
        "release.yml",
        "--limit",
        "10",
        "--json",
        "status,conclusion,headBranch",
        "-q",
        &format!(".[] | select(.headBranch == \"{tag}\") | .conclusion"),
    ])
    .unwrap_or_default();
    let first_conclusion = done.lines().next().unwrap_or("");
    if first_conclusion == "success" {
        println!("  → release workflow already succeeded, skipping");
        return Ok(());
    }
    let prev_id = gh(&[
        "run", "list", "--workflow", "release.yml",
        "--limit", "10",
        "--json", "databaseId,conclusion,headBranch",
        "-q", &format!(".[] | select(.headBranch == \"{tag}\") | select(.conclusion == \"failure\") | .databaseId"),
    ])
    .unwrap_or_default();
    let prev_id_first = prev_id.lines().next().unwrap_or("");
    if !prev_id_first.is_empty() {
        println!("  → previous release run failed, re-running...");
        gh(&["run", "rerun", prev_id_first])?;
    }
    wait_for_workflow(
        "release.yml",
        RunFilter::Branch(tag),
        Duration::from_secs(30 * 60),
    )?;
    println!("  ✅ GitHub Release created");
    Ok(())
}

pub(crate) fn cmd_release() -> Result {
    let root = project_root();
    ensure_clean_tree(&root)?;

    let version = read_workspace_version(&root)?;
    let tag = format!("v{version}");
    println!("  → releasing {tag}");

    let head_sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if head_sha.is_empty() {
        return Err("could not read HEAD sha".into());
    }

    println!("\n  → git push origin main");
    run_cmd("git", &["push", "origin", "main"])?;

    println!(
        "\n  → waiting for CI to pass on {} (timeout 20min)...",
        &head_sha[..7]
    );
    wait_for_workflow(
        "ci.yml",
        RunFilter::Sha(&head_sha),
        Duration::from_secs(20 * 60),
    )?;
    println!("  ✅ CI passed");

    println!("\n  → tagging {tag}");
    tag_and_push(&root, &tag)?;

    println!("\n  → waiting for release workflow (timeout 30min)...");
    wait_for_release_workflow(&tag)?;

    println!("\n  → publishing to crates.io...");
    cmd_publish(false)?;

    println!("\n  🎉 released {tag} successfully!");
    Ok(())
}
