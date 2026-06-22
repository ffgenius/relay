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
    path::Path,
    process::{Command, Stdio},
};

use crate::config::{self, Config, Paths};
use crate::{RelayError, Result};

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

    println!("Creating a secret GitHub Gist...");
    let gist_id = create_gist(&content)?;

    save_sync_state(paths, &SyncState {
        provider: "gist".into(),
        gist_id,
        synced_hash: hash,
    })?;

    println!("✔ Gist created: https://gist.github.com/{gist_id}");
    println!("  Use `relay sync push` to upload, `relay sync pull` to download.");
    println!("  On another machine, run `relay sync init` and paste this Gist ID.");
    Ok(())
}

/// `relay sync push` — upload local config to the configured Gist.
pub fn push(paths: &Paths) -> Result<()> {
    check_gh()?;
    let state = load_sync_state(paths)?;

    let config = config::load(paths)?;
    let content = config_yaml_string(&config)?;
    let hash = sha256_hex(&content);

    println!("Uploading config to Gist {}...", state.gist_id);
    update_gist(&state.gist_id, &content)?;

    save_sync_state(paths, &SyncState {
        synced_hash: hash,
        ..state
    })?;

    println!("✔ Pushed ({} total commands)", config.commands.len());
    Ok(())
}

/// `relay sync pull` — download the Gist and overwrite the local config.
///
/// If the local config has unsynced changes, warn and ask for confirmation
/// before overwriting.
pub fn pull(paths: &Paths) -> Result<()> {
    check_gh()?;
    let state = load_sync_state(paths)?;

    // Detect local drift before overwriting.
    let local_config = config::load(paths)?;
    let local_content = config_yaml_string(&local_config)?;
    let local_hash = sha256_hex(&local_content);

    if local_hash != state.synced_hash && !local_config.commands.is_empty() {
        println!(
            "⚠ Local config has changed since last sync.\n\
             `relay sync pull` will overwrite those changes with the remote version."
        );
        if !prompt_continue() {
            println!("  cancelled.");
            return Ok(());
        }
    }

    println!("Downloading config from Gist {}...", state.gist_id);
    let remote_content = download_gist(&state.gist_id)?;

    // The Gist stores a config.yaml file, parse it.
    let remote_config: Config = serde_yaml::from_str(&remote_content).map_err(|e| {
        RelayError::Other(anyhow::anyhow!("invalid config in Gist: {e}"))
    })?;

    // Write to disk.
    config::save(paths, &remote_config)?;
    let new_hash = sha256_hex(&remote_content);

    save_sync_state(paths, &SyncState {
        synced_hash: new_hash,
        ..state
    })?;

    // Re-sync shims.
    crate::shim::sync(paths, &remote_config)?;

    println!(
        "✔ Pulled ({} commands). Shims regenerated.",
        remote_config.commands.len()
    );
    Ok(())
}

/// `relay sync status` — display whether sync is configured and up to date.
pub fn status(paths: &Paths) -> Result<()> {
    let state_path = sync_state_path(paths);
    if !state_path.exists() {
        println!("sync: not configured");
        println!("  run `relay sync init` to set up GitHub Gist sync.");
        return Ok(());
    }

    let state = load_sync_state(paths)?;
    println!("sync: configured");
    println!("  provider: {}", state.provider);
    println!("  gist id:  {}", state.gist_id);

    let config = config::load(paths)?;
    let content = config_yaml_string(&config)?;
    let local_hash = sha256_hex(&content);

    if local_hash == state.synced_hash {
        println!("  state:    clean (local matches remote)");
    } else {
        println!("  state:    dirty (local has un-pushed changes)");
    }
    println!(
        "  commands: {} ({} prefix, {} exact)",
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
    );

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
        Err(_) | Ok(s) if !s.success() => {
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
        Err(_) | Ok(s) if !s.success() => {
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

    run_gh_api(&[&format!("/gists/{gist_id}"), "--input", "-"], Some(&body.to_string()))?;
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
        let mut child = cmd.spawn().map_err(|e| {
            RelayError::Other(anyhow::anyhow!("failed to spawn gh: {e}"))
        })?;
        if let Some(body) = stdin_body {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(body.as_bytes()).ok();
            }
        }
        let output = child.wait_with_output().map_err(|e| {
            RelayError::Other(anyhow::anyhow!("gh api failed: {e}"))
        })?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(RelayError::Other(anyhow::anyhow!(
                "gh api failed:\n{stderr}"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        cmd.stdout(Stdio::piped());
        let output = cmd.output().map_err(|e| {
            RelayError::Other(anyhow::anyhow!("failed to spawn gh: {e}"))
        })?;
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
    let state: SyncState = serde_yaml::from_slice(&bytes).map_err(|e| {
        RelayError::Other(anyhow::anyhow!("invalid sync-state.yaml: {e}"))
    })?;
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