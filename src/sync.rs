//! Gist sync — `relay sync {init | push | pull | status}`.
//!
//! All GitHub API calls go through `gh api` (the official GitHub CLI).
//! This avoids managing tokens, OAuth flows, or adding HTTP/JSON deps.
//! The user authenticates once with `gh auth login` and relay reuses it.
//!
//! Data flow:
//!
//!   init   create ~/.relay/sync-state.yaml + POST /gists
//!   push   PATCH /gists/{id}  (local → remote)
//!   pull   GET  /gists/{id}   (remote → local)
//!   status show sync state

use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

use crate::config::{self, schema::ShellDialect, Config, Paths};
use crate::{ui, RelayError, Result};

/// Sync state persisted as `~/.relay/sync-state.yaml`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncState {
    pub provider: String,
    pub gist_id: String,
    /// Hex-encoded SHA-256 of config.yaml at last push/pull, used to
    /// detect local drift before overwriting.
    pub synced_hash: String,
}

// ─── Public entry points ────────────────────────────────────────────────

/// `relay sync init` — guide the user through setting up Gist sync.
///
/// 1. Check that `gh` is installed and authenticated.
/// 2. Create a fresh secret Gist containing the current config.
/// 3. Write the Gist ID and hash into sync-state.yaml.
pub fn init(paths: &Paths) -> Result<()> {
    check_gh()?;

    let config = config::load(paths)?;
    let content = config_yaml_string(&config)?;
    let hash = sha256_hex(&content);

    let pb = ui::spinner("Creating a secret GitHub Gist...");
    let gist_id = match create_gist(&content) {
        Ok(id) => id,
        Err(e) => {
            pb.finish_and_clear();
            return Err(e);
        }
    };
    ui::spinner_finish(&pb, format!("Gist created (id: {gist_id})"));

    save_sync_state(
        paths,
        &SyncState {
            provider: "gist".into(),
            gist_id: gist_id.clone(),
            synced_hash: hash,
        },
    )?;

    ui::line(format!("  url: https://gist.github.com/{gist_id}"));
    ui::note("`relay sync push` uploads, `relay sync pull` downloads.");
    ui::note(format!(
        "on another machine, run `relay sync link {gist_id}`."
    ));
    Ok(())
}

/// `relay sync link <gist_id>` — connect to an existing Gist without
/// downloading or uploading. Use on a second machine after getting the
/// Gist ID from `relay sync init` on the first machine.
///
/// After linking, run `relay sync pull` to download the aliases.
pub fn link(paths: &Paths, gist_id: &str) -> Result<()> {
    let pb = ui::spinner("Verifying Gist...");
    // Verify the Gist exists and has a config.yaml file.
    let content = match download_gist(gist_id) {
        Ok(c) => c,
        Err(e) => {
            pb.finish_and_clear();
            return Err(e);
        }
    };
    // Parse to validate it's a config we understand.
    let _config: Config = serde_yaml::from_str(&content).map_err(|e| {
        pb.finish_and_clear();
        RelayError::Other(anyhow::anyhow!(
            "Gist {gist_id} doesn't contain a valid relay config: {e}"
        ))
    })?;
    ui::spinner_finish(&pb, format!("Linked to Gist {gist_id}"));

    let hash = sha256_hex(&content);
    save_sync_state(
        paths,
        &SyncState {
            provider: "gist".into(),
            gist_id: gist_id.to_string(),
            synced_hash: hash,
        },
    )?;

    ui::note("run `relay sync pull` to download aliases.");
    Ok(())
}

/// `relay sync unlink` — forget the Gist link on this machine.
///
/// Removes `~/.relay/sync-state.yaml` so `relay sync push/pull` will
/// refuse to run until the machine is re-linked. **Does not** delete
/// the remote Gist — that's intentional, the Gist still works as a
/// backup and can be re-linked any time via `relay sync link <id>`.
pub fn unlink(paths: &Paths) -> Result<()> {
    let path = sync_state_path(paths);
    if !path.exists() {
        ui::line("sync: not configured — nothing to unlink");
        return Ok(());
    }

    // Read the existing state so we can print a helpful confirmation
    // showing what was unlinked.
    let state = load_sync_state(paths)?;

    fs::remove_file(&path).map_err(|source| RelayError::Io {
        path: path.clone(),
        source,
    })?;

    ui::ok(format!("unlinked from Gist {}", state.gist_id));
    ui::note(format!(
        "the Gist still exists at https://gist.github.com/{}",
        state.gist_id
    ));
    ui::note(format!(
        "re-link any time with `relay sync link {}`.",
        state.gist_id
    ));
    Ok(())
}

/// `relay sync push` — upload local config to the configured Gist.
///
/// By default, snippets are included in the push. Pass `no_snippet = true`
/// to exclude them (only commands will be uploaded).
pub fn push(paths: &Paths, no_snippet: bool) -> Result<()> {
    check_gh()?;
    let state = load_sync_state(paths)?;

    let mut config = config::load(paths)?;
    let snippet_count = config.snippets.len();

    if no_snippet {
        config.snippets.clear();
    }

    let content = config_yaml_string(&config)?;
    let hash = sha256_hex(&content);

    let pb = ui::spinner("Uploading to Gist...");
    if let Err(e) = update_gist(&state.gist_id, &content) {
        pb.finish_and_clear();
        return Err(e);
    }

    let msg = if no_snippet {
        format!(
            "Pushed ({} commands). {} snippet(s) excluded.",
            config.commands.len(),
            snippet_count
        )
    } else if snippet_count > 0 {
        format!(
            "Pushed ({} commands + {} snippets)",
            config.commands.len(),
            snippet_count
        )
    } else {
        format!("Pushed ({} commands)", config.commands.len())
    };
    ui::spinner_finish(&pb, msg);

    save_sync_state(
        paths,
        &SyncState {
            synced_hash: hash,
            ..state
        },
    )?;

    Ok(())
}

/// `relay sync pull` — download the Gist and overwrite the local config.
///
/// If the local config has unsynced changes, warn and ask for confirmation
/// before overwriting.
///
/// Snippets are **skipped by default** for security. Pass
/// `allow_snippet = true` to pull them from the remote Gist.
pub fn pull(paths: &Paths, allow_snippet: bool) -> Result<()> {
    check_gh()?;
    let state = load_sync_state(paths)?;

    // Detect local drift before overwriting.
    let local_config = config::load(paths)?;
    let local_content = config_yaml_string(&local_config)?;
    let local_hash = sha256_hex(&local_content);

    if local_hash != state.synced_hash && !local_config.commands.is_empty() {
        ui::warn(
            "local config has changed since last sync.\n       \
             `relay sync pull` will overwrite those changes with the remote version.",
        );
        if !prompt_continue() {
            ui::line("  cancelled.");
            return Ok(());
        }
    }

    let pb = ui::spinner("Downloading from Gist...");
    let remote_content = match download_gist(&state.gist_id) {
        Ok(c) => c,
        Err(e) => {
            pb.finish_and_clear();
            return Err(e);
        }
    };

    // The Gist stores a config.yaml file, parse it.
    let mut remote_config: Config = serde_yaml::from_str(&remote_content).map_err(|e| {
        pb.finish_and_clear();
        RelayError::Other(anyhow::anyhow!("invalid config in Gist: {e}"))
    })?;

    let remote_snippet_count = remote_config.snippets.len();

    // Strip snippets unless explicitly allowed.
    if !allow_snippet && remote_snippet_count > 0 {
        remote_config.snippets.clear();
    }

    // Write to disk.
    config::save(paths, &remote_config)?;
    let new_hash = sha256_hex(&remote_content);

    save_sync_state(
        paths,
        &SyncState {
            synced_hash: new_hash,
            ..state
        },
    )?;

    // Re-sync shims.
    crate::shim::sync(paths, &remote_config)?;

    let msg = if allow_snippet {
        format!(
            "Pulled ({} commands + {} snippets). Shims regenerated.",
            remote_config.commands.len(),
            remote_snippet_count
        )
    } else {
        let extra = if remote_snippet_count > 0 {
            format!(
                " {} snippet(s) not pulled. Use --allow-snippet to include them.",
                remote_snippet_count
            )
        } else {
            String::new()
        };
        format!(
            "Pulled ({} commands).{} Shims regenerated.",
            remote_config.commands.len(),
            extra
        )
    };
    ui::spinner_finish(&pb, msg);

    if !allow_snippet && remote_snippet_count > 0 {
        ui::warn(format!(
            "{} snippet(s) not pulled. Use --allow-snippet to include them.",
            remote_snippet_count
        ));
    }

    Ok(())
}

/// `relay sync status` — display whether sync is configured and up to date.
pub fn status(paths: &Paths) -> Result<()> {
    let state_path = sync_state_path(paths);
    if !state_path.exists() {
        ui::line("sync: not configured");
        ui::note("run `relay sync init` to set up GitHub Gist sync.");
        return Ok(());
    }

    let state = load_sync_state(paths)?;
    ui::line("sync: configured");
    ui::field("provider", &state.provider);
    ui::field("gist id", &state.gist_id);

    let config = config::load(paths)?;
    let content = config_yaml_string(&config)?;
    let local_hash = sha256_hex(&content);

    if local_hash == state.synced_hash {
        ui::ok("state: clean (local matches remote)");
    } else {
        ui::warn("state: dirty (local has un-pushed changes)");
    }
    ui::field(
        "commands",
        format!(
            "{} ({} prefix, {} exact)",
            config.commands.len(),
            config
                .commands
                .values()
                .filter(|c| matches!(c.kind, config::CommandKind::Prefix))
                .count(),
            config
                .commands
                .values()
                .filter(|c| matches!(c.kind, config::CommandKind::Exact))
                .count(),
        ),
    );

    if !config.snippets.is_empty() {
        let unix_count = config
            .snippets
            .values()
            .filter(|s| s.shell == ShellDialect::Unix)
            .count();
        let ps_count = config
            .snippets
            .values()
            .filter(|s| s.shell == ShellDialect::PowerShell)
            .count();
        let cmd_count = config
            .snippets
            .values()
            .filter(|s| s.shell == ShellDialect::Cmd)
            .count();
        let mut parts = Vec::new();
        if unix_count > 0 {
            parts.push(format!("{unix_count} unix"));
        }
        if ps_count > 0 {
            parts.push(format!("{ps_count} powershell"));
        }
        if cmd_count > 0 {
            parts.push(format!("{cmd_count} cmd"));
        }
        ui::field("snippets", parts.join(", "));
    } else {
        ui::field("snippets", "0");
    }

    Ok(())
}

// ─── GitHub API helpers (via `gh api`) ──────────────────────────────────

/// Check that `gh` is installed and authenticated.
/// If not, print a clear action message (don't exit silently).
fn check_gh() -> Result<()> {
    let which = Command::new("gh")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match which {
        Err(_) => {
            return Err(RelayError::Other(anyhow::anyhow!(
                "gh (GitHub CLI) not found on PATH.\n\
                 Install it from https://cli.github.com/ then run:\n\n    \
                 gh auth login\n\n\
                 After that, try `relay sync init` again."
            )));
        }
        Ok(s) if !s.success() => {
            return Err(RelayError::Other(anyhow::anyhow!(
                "gh (GitHub CLI) not found on PATH.\n\
                 Install it from https://cli.github.com/ then run:\n\n    \
                 gh auth login\n\n\
                 After that, try `relay sync init` again."
            )));
        }
        _ => {}
    }

    let auth = Command::new("gh")
        .args(["auth", "status"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match auth {
        Err(_) => {
            return Err(RelayError::Other(anyhow::anyhow!(
                "gh is installed but not authenticated.\n\
                 Run:\n\n    \
                 gh auth login\n\n\
                 Then try `relay sync init` again."
            )));
        }
        Ok(s) if !s.success() => {
            return Err(RelayError::Other(anyhow::anyhow!(
                "gh is installed but not authenticated.\n\
                 Run:\n\n    \
                 gh auth login\n\n\
                 Then try `relay sync init` again."
            )));
        }
        _ => {}
    }

    Ok(())
}

/// Create a *secret* Gist with one file (`config.yaml`) and return its ID.
/// Calls `POST /gists` with the JSON body piped via stdin.
fn create_gist(content: &str) -> Result<String> {
    let body = serde_json::json!({
        "description": "relay command aliases (synced by relay)",
        "public": false,
        "files": {
            "config.yaml": {
                "content": content
            }
        }
    });

    let output = run_gh_api(&["/gists", "--input", "-"], Some(&body.to_string()))?;
    // Parse the ID from the JSON response.
    let parsed: serde_json::Value =
        serde_json::from_str(&output).map_err(|e| RelayError::Other(e.into()))?;
    let gist_id = parsed["id"]
        .as_str()
        .ok_or_else(|| {
            RelayError::Other(anyhow::anyhow!(
                "unexpected response from GitHub API: missing 'id'"
            ))
        })?
        .to_string();
    Ok(gist_id)
}

/// Update an existing Gist's `config.yaml` file. Uses `PATCH /gists/{id}`.
fn update_gist(gist_id: &str, content: &str) -> Result<()> {
    let body = serde_json::json!({
        "files": {
            "config.yaml": {
                "content": content
            }
        }
    });

    run_gh_api(
        &[&format!("/gists/{gist_id}"), "--input", "-"],
        Some(&body.to_string()),
    )?;
    Ok(())
}

/// Download the `config.yaml` file from a Gist. Returns the raw content.
fn download_gist(gist_id: &str) -> Result<String> {
    let output = run_gh_api(&[&format!("/gists/{gist_id}")], None)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&output).map_err(|e| RelayError::Other(e.into()))?;

    // Navigate: files -> config.yaml -> content
    let content = parsed["files"]["config.yaml"]["content"]
        .as_str()
        .ok_or_else(|| {
            RelayError::Other(anyhow::anyhow!(
                "Gist {gist_id} does not contain a 'config.yaml' file"
            ))
        })?
        .to_string();
    Ok(content)
}

/// Run `gh api <args>` and return stdout. Optionally pipe a JSON body via stdin.
fn run_gh_api(args: &[&str], stdin_body: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("gh");
    cmd.args(["api"]);
    cmd.args(args);
    cmd.stderr(Stdio::piped());

    if stdin_body.is_some() {
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| RelayError::Other(anyhow::anyhow!("failed to spawn gh: {e}")))?;
        if let Some(body) = stdin_body {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(body.as_bytes()).ok();
            }
        }
        let output = child
            .wait_with_output()
            .map_err(|e| RelayError::Other(anyhow::anyhow!("gh api failed: {e}")))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(RelayError::Other(anyhow::anyhow!(
                "gh api failed:\n{stderr}"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        cmd.stdout(Stdio::piped());
        let output = cmd
            .output()
            .map_err(|e| RelayError::Other(anyhow::anyhow!("failed to spawn gh: {e}")))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(RelayError::Other(anyhow::anyhow!(
                "gh api failed:\n{stderr}"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

// ─── Local state persistence ────────────────────────────────────────────

fn sync_state_path(paths: &Paths) -> std::path::PathBuf {
    paths.root().join("sync-state.yaml")
}

fn save_sync_state(paths: &Paths, state: &SyncState) -> Result<()> {
    let path = sync_state_path(paths);
    let yaml = serde_yaml::to_string(state).map_err(|e| RelayError::Other(e.into()))?;
    fs::write(&path, yaml).map_err(|source| RelayError::Io { path, source })?;
    Ok(())
}

fn load_sync_state(paths: &Paths) -> Result<SyncState> {
    let path = sync_state_path(paths);
    let bytes = fs::read(&path).map_err(|source| RelayError::Io {
        path: path.clone(),
        source,
    })?;
    let state: SyncState = serde_yaml::from_slice(&bytes)
        .map_err(|e| RelayError::Other(anyhow::anyhow!("invalid sync-state.yaml: {e}")))?;
    Ok(state)
}

// ─── Utilities ──────────────────────────────────────────────────────────

/// Serialize a Config as YAML string.
fn config_yaml_string(config: &Config) -> Result<String> {
    serde_yaml::to_string(config).map_err(|e| RelayError::Other(e.into()))
}

/// Hex-encoded SHA-256 of a string.
fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in hash {
        write!(out, "{byte:02x}").unwrap();
    }
    out
}

/// Prompt the user for a yes/no answer via stdin. Empty/yes = proceed.
fn prompt_continue() -> bool {
    use std::io::BufRead;
    print!("  Continue? [y/N] ");
    std::io::stdout().flush().ok();
    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();
    let trimmed = line.trim().to_lowercase();
    trimmed == "y" || trimmed == "yes"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_is_deterministic() {
        assert_eq!(sha256_hex("relay"), sha256_hex("relay"));
    }

    #[test]
    fn sha256_hex_differs_for_different_input() {
        assert_ne!(sha256_hex("relay"), sha256_hex("relay-sync"));
    }

    #[test]
    fn sync_state_roundtrip() {
        let state = SyncState {
            provider: "gist".into(),
            gist_id: "abc123".into(),
            synced_hash: "deadbeef".into(),
        };
        let yaml = serde_yaml::to_string(&state).unwrap();
        let parsed: SyncState = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.gist_id, "abc123");
        assert_eq!(parsed.provider, "gist");
    }
}
