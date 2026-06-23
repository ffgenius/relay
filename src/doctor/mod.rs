//! `relay doctor` — validate the environment.
//!
//! Per product.md the four checks are:
//!   1. Relay PATH       — is `~/.relay/bin` on $PATH?
//!   2. Command PATH     — is every registered program resolvable on $PATH?
//!   3. Shim status      — does every registered command have a shim file?
//!   4. Config integrity — does config.yaml parse cleanly?
//!
//! With `fix=true`, item 3 (shim mismatch) is repaired by re-running
//! `shim::sync`. PATH mismatches are reported but not edited — that touches
//! shell-profile files and is out of scope for v0.1.

use std::path::PathBuf;

use crate::{
    config::{self, Paths},
    path_setup::{self, InstallOutcome},
    shim, ui, Result,
};

pub fn run(paths: &Paths, fix: bool) -> Result<()> {
    ui::header("relay root :", paths.root().display());
    ui::header("config file:", paths.config_file().display());
    ui::header("shim dir   :", paths.bin_dir().display());
    ui::blank();

    let mut issues = 0usize;

    // ── 4. Config integrity ────────────────────────────────────────────
    // Done first so the rest can lean on a parsed Config.
    let config = match config::load(paths) {
        Ok(c) => {
            ui::ok(format!(
                "config parses cleanly ({} commands)",
                c.commands.len()
            ));
            c
        }
        Err(e) => {
            ui::err(format!("config failed to parse: {e}"));
            // Without a config the remaining checks can't proceed.
            return Ok(());
        }
    };

    // ── 1. Relay PATH ──────────────────────────────────────────────────
    let bin_dir = paths.bin_dir();
    let path_ok = path_contains(&bin_dir);
    if path_ok {
        ui::ok("shim dir is on PATH");
    } else {
        ui::warn(format!(
            "shim dir is NOT on PATH — add {} to your shell profile",
            bin_dir.display()
        ));
        issues += 1;
    }

    // ── 2. Command PATH ────────────────────────────────────────────────
    let mut missing_programs = Vec::new();
    for (name, cmd) in &config.commands {
        match which::which(&cmd.program) {
            Ok(_) => ui::ok(format!("program {name} -> {} on PATH", cmd.program)),
            Err(_) => {
                ui::warn(format!("program {name} -> {} NOT on PATH", cmd.program));
                missing_programs.push(name.clone());
                issues += 1;
            }
        }
    }

    // ── 3. Shim status ─────────────────────────────────────────────────
    let mut shim_issues = Vec::new();
    for name in config.commands.keys() {
        let target = bin_dir.join(platform_shim_file(name));
        if target.exists() {
            ui::ok(format!("shim {name} present"));
        } else {
            ui::warn(format!("shim {name} missing"));
            shim_issues.push(name.clone());
            issues += 1;
        }
    }
    // Stray shims that don't correspond to any command.
    if let Ok(rd) = std::fs::read_dir(&bin_dir) {
        for entry in rd.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            let stripped = strip_shim_ext(&fname);
            if !config.commands.contains_key(stripped) {
                ui::warn(format!("orphan shim file: {fname}"));
                shim_issues.push(stripped.to_string());
                issues += 1;
            }
        }
    }

    ui::blank();
    if issues == 0 {
        ui::line("all good.");
        return Ok(());
    }

    if fix {
        if !shim_issues.is_empty() {
            ui::line(format!("fixing {} shim issue(s)...", shim_issues.len()));
            shim::sync(paths, &config)?;
        }
        if !path_ok {
            // Skip the system-level PATH edit when running against a
            // sandboxed root (tests / `--root <tmpdir>`) — only the real
            // `~/.relay` should ever touch the user's environment.
            if !is_default_root(paths) {
                ui::warn(format!(
                    "PATH not modified — running with a non-default root.\n       add `{}` to your PATH manually",
                    bin_dir.display()
                ));
            } else {
                ui::line("fixing PATH...");
                match path_setup::install(paths) {
                    InstallOutcome::AlreadyPresent => {
                        ui::ok("shim dir already on PATH");
                    }
                    InstallOutcome::Installed => {
                        ui::ok("shim dir added to PATH");
                        ui::note("open a new terminal for it to take effect");
                    }
                    InstallOutcome::Unsupported(reason) | InstallOutcome::Failed(reason) => {
                        ui::warn(format!("could not auto-update PATH: {reason}"));
                        ui::line(format!(
                            "       add `{}` to your PATH manually",
                            bin_dir.display()
                        ));
                    }
                }
            }
        }
        ui::line("done. re-run `relay doctor` to verify.");
    } else {
        ui::line(format!(
            "{issues} issue(s) found. re-run with --fix to repair."
        ));
    }

    Ok(())
}

/// True iff `paths` resolves to the platform default `~/.relay`.
/// Used to suppress system-level PATH edits under a `--root <tmpdir>`
/// override so tests cannot pollute the user's HKCU / shell profile.
fn is_default_root(paths: &Paths) -> bool {
    let Ok(default) = config::Paths::discover() else {
        return false;
    };
    paths.root() == default.root()
}

/// Is the given directory present in $PATH? Comparison is by canonicalised
/// path where possible, falling back to string equality otherwise.
fn path_contains(dir: &std::path::Path) -> bool {
    let want = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for entry in std::env::split_paths(&path_var) {
        let candidate: PathBuf = std::fs::canonicalize(&entry).unwrap_or(entry);
        if candidate == want {
            return true;
        }
    }
    false
}

#[cfg(windows)]
fn platform_shim_file(name: &str) -> String {
    format!("{name}.cmd")
}

#[cfg(not(windows))]
fn platform_shim_file(name: &str) -> String {
    name.to_string()
}

#[cfg(windows)]
fn strip_shim_ext(file: &str) -> &str {
    file.strip_suffix(".cmd").unwrap_or(file)
}

#[cfg(not(windows))]
fn strip_shim_ext(file: &str) -> &str {
    file
}
