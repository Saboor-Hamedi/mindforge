#!/usr/bin/env rust-script
//! MindForge publish automation.
//!
//! Usage: `cargo run --bin publish`
//!
//! What this does, end-to-end:
//!  1. Verify the git working tree is clean.
//!  2. Run the full workspace test suite.
//!  3. Read the current version from Cargo.toml.
//!  4. Bump the PATCH version (or accept a version argument).
//!  5. Write the new version back to app/Cargo.toml.
//!  6. `cargo check --workspace` to ensure it compiles.
//!  7. `git add -A && git commit -m "chore: release vX.Y.Z"`.
//!  8. `git tag vX.Y.Z`.
//!  9. `git push && git push --tags` → triggers the GitHub Actions release workflow.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

// ─── Colours (ANSI) ──────────────────────────────────────────────────────────
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

fn step(msg: &str) {
    println!("\n{BOLD}{CYAN}▶  {msg}{RESET}");
}

fn ok(msg: &str) {
    println!("{GREEN}✓  {msg}{RESET}");
}

#[allow(dead_code)]
fn warn(msg: &str) {
    println!("{YELLOW}⚠  {msg}{RESET}");
}

fn bail(msg: &str) -> ! {
    eprintln!("{RED}{BOLD}✗  {msg}{RESET}");
    std::process::exit(1);
}

// ─── Shell helpers ────────────────────────────────────────────────────────────
fn run(program: &str, args: &[&str], cwd: &Path) {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|e| bail(&format!("failed to execute {program}: {e}")));

    if !status.success() {
        bail(&format!(
            "`{program} {}` exited with status {status}",
            args.join(" ")
        ));
    }
}

#[allow(dead_code)]
fn run_output(program: &str, args: &[&str], cwd: &Path) -> String {
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| bail(&format!("failed to run {program}: {e}")));

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail(&format!("`{program} {}` failed: {stderr}", args.join(" ")));
    }

    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

// ─── Version helpers ──────────────────────────────────────────────────────────
fn read_version(cargo_toml: &Path) -> (String, String) {
    let content = fs::read_to_string(cargo_toml)
        .unwrap_or_else(|e| bail(&format!("Cannot read {:?}: {e}", cargo_toml)));

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let ver = val.trim().trim_matches('"').to_string();
                return (ver, content);
            }
        }
    }
    bail("Could not find `version` field in Cargo.toml");
}

fn bump_patch(version: &str) -> String {
    let parts: Vec<u64> = version.split('.').filter_map(|p| p.parse().ok()).collect();

    if parts.len() != 3 {
        bail(&format!("Unexpected version format: {version}"));
    }
    format!("{}.{}.{}", parts[0], parts[1], parts[2] + 1)
}

fn write_version(cargo_toml: &Path, content: &str, old: &str, new: &str) {
    // Replace only the first occurrence of the package version line
    let new_content = content.replacen(
        &format!("version = \"{old}\""),
        &format!("version = \"{new}\""),
        1,
    );
    fs::write(cargo_toml, new_content)
        .unwrap_or_else(|e| bail(&format!("Cannot write {:?}: {e}", cargo_toml)));
}

// ─── Git helpers ──────────────────────────────────────────────────────────────
fn git_is_clean(repo: &Path) -> bool {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .unwrap_or_else(|e| bail(&format!("git status failed: {e}")));
    out.stdout.is_empty()
}

fn tag_exists(repo: &Path, tag: &str) -> bool {
    let out = Command::new("git")
        .args(["tag", "-l", tag])
        .current_dir(repo)
        .output()
        .unwrap_or_else(|e| bail(&format!("git tag failed: {e}")));
    !String::from_utf8_lossy(&out.stdout).trim().is_empty()
}

// ─── Main ────────────────────────────────────────────────────────────────────
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Resolve workspace root (parent of app/)
    // When run via `cargo run --bin publish`, the cwd is the workspace root.
    let cwd = std::env::current_dir().expect("cannot get cwd");

    // The app's Cargo.toml
    let app_cargo = cwd.join("app").join("Cargo.toml");
    if !app_cargo.exists() {
        bail(&format!(
            "app/Cargo.toml not found — run this from the workspace root. cwd={cwd:?}"
        ));
    }

    println!("\n{BOLD}╔══════════════════════════════════════════╗");
    println!("║     MindForge Publish Automation         ║");
    println!("╚══════════════════════════════════════════╝{RESET}\n");

    // ── 1. Read current version ───────────────────────────────────────────────
    step("Reading current version from app/Cargo.toml");
    let (current_ver, content) = read_version(&app_cargo);
    ok(&format!("Current version: v{current_ver}"));

    // ── 2. Determine new version ──────────────────────────────────────────────
    let new_ver = if args.len() > 1 && args[1].starts_with(|c: char| c.is_ascii_digit()) {
        args[1].clone()
    } else {
        let bumped = bump_patch(&current_ver);
        print!(
            "\n{YELLOW}New version [{bumped}]? (press Enter to accept, or type a version): {RESET}"
        );
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let trimmed = input.trim();
        if trimmed.is_empty() {
            bumped
        } else {
            trimmed.to_string()
        }
    };
    ok(&format!("Publishing version: v{new_ver}"));

    // ── 3. Verify git is clean (before bumping) ───────────────────────────────
    step("Checking working tree is clean");
    if !git_is_clean(&cwd) {
        // Show the diff so user knows what's uncommitted
        let _ = Command::new("git")
            .args(["status", "--short"])
            .current_dir(&cwd)
            .status();
        bail("Working tree has uncommitted changes — commit or stash them first.");
    }
    ok("Working tree is clean");

    // ── 4. Check tag doesn't already exist ────────────────────────────────────
    let tag = format!("v{new_ver}");
    if tag_exists(&cwd, &tag) {
        bail(&format!(
            "Tag {tag} already exists. Bump to a higher version."
        ));
    }

    // ── 5. Run workspace tests ────────────────────────────────────────────────
    step("Running workspace tests (cargo test --workspace)");
    run("cargo", &["test", "--workspace"], &cwd);
    ok("All tests passed");

    // ── 6. Write bumped version ───────────────────────────────────────────────
    step(&format!("Bumping version: {current_ver} → {new_ver}"));
    write_version(&app_cargo, &content, &current_ver, &new_ver);
    ok(&format!("Wrote v{new_ver} to app/Cargo.toml"));

    // Also update workspace Cargo.lock by touching it via cargo check
    step("Updating Cargo.lock (cargo check)");
    run("cargo", &["check", "--workspace", "--quiet"], &cwd);
    ok("Cargo.lock updated");

    // ── 7. Git commit ─────────────────────────────────────────────────────────
    step("Committing version bump");
    run("git", &["add", "app/Cargo.toml", "Cargo.lock"], &cwd);
    run(
        "git",
        &["commit", "-m", &format!("chore: release v{new_ver}")],
        &cwd,
    );
    ok(&format!("Committed: chore: release v{new_ver}"));

    // ── 8. Git tag ────────────────────────────────────────────────────────────
    step(&format!("Tagging release: {tag}"));
    run(
        "git",
        &["tag", "-a", &tag, "-m", &format!("Release {tag}")],
        &cwd,
    );
    ok(&format!("Created annotated tag {tag}"));

    // ── 9. Push ───────────────────────────────────────────────────────────────
    step("Pushing commits and tag to GitHub");
    run("git", &["push"], &cwd);
    run("git", &["push", "--tags"], &cwd);
    ok("Pushed to GitHub — CI/CD pipeline is now running!");

    println!("\n{BOLD}{GREEN}════════════════════════════════════════");
    println!("  ✓  Release v{new_ver} published!");
    println!("  GitHub Actions will now:");
    println!("  • Build on Windows, macOS, Linux");
    println!("  • Create a GitHub Release with binaries");
    println!("  • Upload installers (.exe, .dmg, .deb)");
    println!("════════════════════════════════════════{RESET}\n");

    println!("  View your release at:");
    println!("  {CYAN}https://github.com/Saboor-Hamedi/mindforge/releases/tag/{tag}{RESET}\n");
}
