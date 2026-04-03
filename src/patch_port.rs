use dialoguer::console::style;
use dialoguer::{FuzzySelect, Input, Select, theme::ColorfulTheme};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn move_patch_to_llvm_release(llvm_dir: &PathBuf) -> bool {
    if cherry_pick_in_progress(llvm_dir) {
        println!(
            "{} Cherry-pick already in progress in {}",
            style("⚠").yellow().bold(),
            llvm_dir.display()
        );
        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("How to handle the ongoing cherry-pick?")
            .items(&[
                "Continue cherry-pick (after resolving conflicts)",
                "Abort cherry-pick",
                "Cancel",
            ])
            .default(2)
            .interact_opt()
            .expect("failed to render cherry-pick recovery menu");

        match choice {
            Some(0) => {
                if !run_git(
                    llvm_dir,
                    &["cherry-pick", "--continue"],
                    "git cherry-pick --continue",
                ) {
                    eprintln!("Cherry-pick continue failed. Resolve conflicts, then retry.");
                }
            }
            Some(1) => {
                let _ = run_git(llvm_dir, &["cherry-pick", "--abort"], "git cherry-pick --abort");
            }
            _ => {}
        }
        return false;
    }

    if !ensure_clean_worktree(llvm_dir) {
        eprintln!(
            "Please commit/stash tracked llvm-project changes first (untracked files are allowed), then retry."
        );
        return false;
    }

    let current_branch = match git_output(llvm_dir, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("failed to detect current branch: {e}");
            return false;
        }
    };
    if current_branch == "HEAD" {
        eprintln!("Detached HEAD detected. Please checkout your patch branch first.");
        return false;
    }

    println!("Current patch branch: {}", style(&current_branch).bold());
    println!("Fetching LLVM refs and tags...");
    if !run_git(
        llvm_dir,
        &["fetch", "--all", "--tags"],
        "git fetch --all --tags",
    ) {
        return false;
    }

    let Some(target_ref) = select_target_ref(llvm_dir) else {
        return false;
    };

    let Some((base_ref, merge_base, commits_to_port)) =
        select_patch_base(llvm_dir, &current_branch)
    else {
        return false;
    };

    if commits_to_port.is_empty() {
        eprintln!(
            "No patch commits found on '{}' relative to '{}'.",
            current_branch, base_ref
        );
        return false;
    }

    println!(
        "Found {} commits to port from '{}' onto target release.",
        style(commits_to_port.len()).bold(),
        style(&current_branch).bold()
    );
    let preview_range = format!("{merge_base}..{current_branch}");
    if let Ok(preview) = git_output(
        llvm_dir,
        &[
            "log",
            "--oneline",
            "--no-merges",
            "--reverse",
            "--max-count",
            "8",
            &preview_range,
        ],
    ) {
        if !preview.trim().is_empty() {
            println!("\nCommits to port (first up to 8):");
            for line in preview.lines() {
                println!("  {line}");
            }
            if commits_to_port.len() > 8 {
                println!("  ...");
            }
            println!();
        }
    }

    let version_label = extract_llvm_version_label(&target_ref).unwrap_or_else(|| "x".to_string());
    let default_branch = format!(
        "llvm-{}-{}",
        version_label,
        sanitize_branch_name(&current_branch)
    );

    let new_branch: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Name for the new ported branch")
        .default(default_branch)
        .interact_text()
        .expect("failed to read branch name");

    let new_branch = new_branch.trim().to_string();
    if new_branch.is_empty() {
        eprintln!("Branch name cannot be empty.");
        return false;
    }
    if new_branch == current_branch {
        eprintln!("New branch name matches current branch. Please choose a different name.");
        return false;
    }

    let branch_ref = format!("refs/heads/{new_branch}");
    if ref_exists(llvm_dir, &branch_ref) {
        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Branch '{new_branch}' already exists"))
            .items(&[
                "Reset and recreate it from target release",
                "Abort",
            ])
            .default(1)
            .interact_opt()
            .expect("failed to render branch overwrite prompt");
        if choice != Some(0) {
            return false;
        }
    }

    println!(
        "Creating '{}' from '{}'...",
        style(&new_branch).bold(),
        style(&target_ref).bold()
    );
    let checkout_status = Command::new("git")
        .args(["checkout", "-B", &new_branch, &target_ref])
        .current_dir(llvm_dir)
        .status();
    match checkout_status {
        Ok(s) if s.success() => {}
        Ok(_) => {
            eprintln!("Failed to create branch '{new_branch}' from '{target_ref}'.");
            return false;
        }
        Err(e) => {
            eprintln!("failed to run git checkout -B: {e}");
            return false;
        }
    }

    println!(
        "Cherry-picking {} commits onto '{}'...",
        style(commits_to_port.len()).bold(),
        style(&new_branch).bold()
    );

    for (idx, commit) in commits_to_port.iter().enumerate() {
        println!(
            "  [{} / {}] cherry-pick {}",
            idx + 1,
            commits_to_port.len(),
            &commit[..commit.len().min(12)]
        );
        let status = Command::new("git")
            .args(["cherry-pick", commit])
            .current_dir(llvm_dir)
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(_) => {
                eprintln!("\nCherry-pick failed on commit {commit}.");
                if let Ok(conflicts) = git_lines(llvm_dir, &["diff", "--name-only", "--diff-filter=U"]) {
                    if !conflicts.is_empty() {
                        eprintln!("Conflicting files:");
                        for file in conflicts {
                            eprintln!("  {file}");
                        }
                    }
                }

                let choice = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("How to proceed?")
                    .items(&[
                        "Resolve manually and continue outside llvm-airfryer (recommended)",
                        "Skip this commit and continue porting",
                        "Abort porting (keep new branch checked out)",
                        "Abort porting and return to original branch",
                    ])
                    .default(0)
                    .interact_opt()
                    .expect("failed to render conflict resolution menu");

                match choice {
                    Some(1) => {
                        if !run_git(llvm_dir, &["cherry-pick", "--skip"], "git cherry-pick --skip") {
                            return false;
                        }
                        println!("  {} Skipped commit and continuing...", style("✔").green());
                        continue;
                    }
                    Some(2) => {
                        let _ = run_git(llvm_dir, &["cherry-pick", "--abort"], "git cherry-pick --abort");
                        eprintln!("Porting aborted. You're still on branch '{}'.", new_branch);
                        return false;
                    }
                    Some(3) => {
                        let _ = run_git(llvm_dir, &["cherry-pick", "--abort"], "git cherry-pick --abort");
                        let _ = run_git(llvm_dir, &["checkout", &current_branch], "git checkout original branch");
                        eprintln!("Porting aborted. Switched back to '{}'.", current_branch);
                        return false;
                    }
                    _ => {
                        eprintln!("Resolve conflicts in {} and continue with:", llvm_dir.display());
                        eprintln!("  git cherry-pick --continue");
                        eprintln!("Or abort with:");
                        eprintln!("  git cherry-pick --abort");
                        return false;
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to run git cherry-pick: {e}");
                return false;
            }
        }
    }

    println!("\n{} Patch port complete!", style("✔").green().bold());
    println!(
        "  New branch: {}",
        style(&new_branch).bold()
    );
    println!(
        "  Based on:   {}",
        style(&target_ref).dim()
    );
    println!(
        "  Ported:     {} commits from {}",
        style(commits_to_port.len()).bold(),
        style(&current_branch).dim()
    );

    true
}

fn ensure_clean_worktree(repo_dir: &PathBuf) -> bool {
    match git_output(repo_dir, &["status", "--porcelain", "--untracked-files=no"]) {
        Ok(s) => s.trim().is_empty(),
        Err(_) => false,
    }
}

fn cherry_pick_in_progress(repo_dir: &PathBuf) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "-q", "CHERRY_PICK_HEAD"])
        .current_dir(repo_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn select_patch_base(
    llvm_dir: &PathBuf,
    current_branch: &str,
) -> Option<(String, String, Vec<String>)> {
    let mut best: Option<(String, String, Vec<String>)> = None;
    for candidate in candidate_base_refs(llvm_dir) {
        if let Some((merge_base, commits)) = collect_patch_commits(llvm_dir, &candidate, current_branch)
        {
            let better = match &best {
                None => true,
                Some((_, _, best_commits)) => commits.len() < best_commits.len(),
            };
            if better {
                best = Some((candidate, merge_base, commits));
            }
        }
    }
    if let Some((base_ref, merge_base, commits)) = best {
        println!(
            "Auto-selected patch base: {} ({} commits to port)",
            style(&base_ref).bold(),
            style(commits.len()).bold()
        );
        return Some((base_ref, merge_base, commits));
    }

    let base_ref = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Patch base ref (e.g. origin/main)")
        .default("origin/main".to_string())
        .interact_text()
        .expect("failed to read base ref")
        .trim()
        .to_string();

    let (merge_base, commits) = collect_patch_commits(llvm_dir, &base_ref, current_branch)?;
    Some((base_ref, merge_base, commits))
}

fn collect_patch_commits(
    llvm_dir: &PathBuf,
    base_ref: &str,
    current_branch: &str,
) -> Option<(String, Vec<String>)> {
    let merge_base = git_output(llvm_dir, &["merge-base", base_ref, current_branch]).ok()?;
    let range = format!("{merge_base}..{current_branch}");
    let commits = git_lines(llvm_dir, &["rev-list", "--reverse", "--no-merges", &range]).ok()?;
    Some((merge_base, commits))
}

fn select_target_ref(llvm_dir: &PathBuf) -> Option<String> {
    let refs = collect_target_refs(llvm_dir);
    if refs.is_empty() {
        let custom: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Target LLVM release tag/ref (e.g. llvmorg-21.1.8)")
            .default("llvmorg-21.1.8".to_string())
            .interact_text()
            .expect("failed to read target ref");
        let custom = custom.trim().to_string();
        if custom.is_empty() {
            return None;
        }
        return Some(custom);
    }

    let mut items = refs.clone();
    items.push("Enter custom branch/tag ref...".to_string());
    let idx = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Target LLVM release tag (type to search)")
        .items(&items)
        .default(0)
        .interact_opt()
        .expect("failed to select target ref");

    let Some(idx) = idx else { return None };
    if idx == refs.len() {
        let custom: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Custom target ref")
            .default(refs[0].clone())
            .interact_text()
            .expect("failed to read custom target ref");
        let custom = custom.trim().to_string();
        if custom.is_empty() {
            return None;
        }
        Some(custom)
    } else {
        Some(refs[idx].clone())
    }
}

fn collect_target_refs(llvm_dir: &PathBuf) -> Vec<String> {
    let mut refs = Vec::new();
    let mut seen = HashSet::new();

    // Prefer official llvmorg release tags.
    if let Ok(tags) = git_lines(llvm_dir, &["tag", "--list", "llvmorg-*", "--sort=-version:refname"])
    {
        for tag in tags.into_iter().take(120) {
            if seen.insert(tag.clone()) {
                refs.push(tag);
            }
        }
    }

    if !refs.is_empty() {
        return refs;
    }

    // Fallback: release branches, if tags are unavailable.
    if let Ok(remote_branches) = git_lines(
        llvm_dir,
        &["branch", "-r", "--format=%(refname:short)", "--sort=-committerdate"],
    ) {
        for b in remote_branches {
            if b.contains("/release/") && seen.insert(b.clone()) {
                refs.push(b);
            }
        }
    }

    refs
}

fn candidate_base_refs(llvm_dir: &PathBuf) -> Vec<String> {
    let mut refs = Vec::new();
    for r in [
        "origin/main",
        "main",
        "origin/master",
        "master",
        "upstream/main",
        "upstream/master",
    ] {
        if ref_exists(llvm_dir, r) {
            refs.push(r.to_string());
        }
    }
    refs
}

fn ref_exists(llvm_dir: &PathBuf, reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .current_dir(llvm_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn extract_llvm_version_label(target_ref: &str) -> Option<String> {
    if let Some(idx) = target_ref.find("/release/") {
        let release_part = &target_ref[idx + "/release/".len()..];
        let major: String = release_part
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !major.is_empty() {
            return Some(major);
        }
    }

    if let Some(stripped) = target_ref.strip_prefix("llvmorg-") {
        let major: String = stripped
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !major.is_empty() {
            return Some(major);
        }
    }

    target_ref
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn sanitize_branch_name(branch: &str) -> String {
    branch.replace('/', "-")
}

fn run_git(repo_dir: &PathBuf, args: &[&str], context: &str) -> bool {
    let status = match Command::new("git").args(args).current_dir(repo_dir).status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to run {context}: {e}");
            return false;
        }
    };
    if !status.success() {
        eprintln!("{context} failed");
        return false;
    }
    true
}

fn git_output(repo_dir: &PathBuf, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .output()
        .map_err(|e| format!("git {} failed to start: {e}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if msg.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            msg
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_lines(repo_dir: &PathBuf, args: &[&str]) -> Result<Vec<String>, String> {
    let out = git_output(repo_dir, args)?;
    Ok(out
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect())
}
