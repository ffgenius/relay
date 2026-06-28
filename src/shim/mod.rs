//! Shim generation and synchronisation.
//!
//! Each registered command gets a tiny launcher under `~/.relay/bin/<name>`
//! that does nothing but `exec relay run <name> "$@"` (unix) or
//! `relay run <name> %*` (Windows .cmd).
//!
//! The public entry point is [`sync`], which reads the config and reconciles
//! the contents of the bin directory in one pass:
//!
//!   - files that have no matching command are removed (only relay-owned ones),
//!   - commands that have no matching file are written.
//!
//! This makes add/remove/update all converge through the same function.

use std::{fs, io::Write, path::Path};

use crate::{
    config::{Config, Paths},
    RelayError, Result,
};

// ── Magic markers used to recognise relay-owned shim files ──────────────

#[cfg(not(windows))]
const MAGIC: &str = "# relay-shim v1";

#[cfg(windows)]
const MAGIC: &str = "REM relay-shim v1";

// ── Shim kind ───────────────────────────────────────────────────────────

/// Distinguishes command shims from snippet shims so the launcher runs the
/// correct relay subcommand (`relay run` vs `relay snippet run`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShimKind {
    /// Regular alias shim → `relay run <name>`.
    Command,
    /// Snippet shim → `relay snippet run <name>`.
    Snippet,
}

// ── Public API ──────────────────────────────────────────────────────────

/// Ensure the bin directory exists, then reconcile shim files against
/// `config.commands` **and** `config.snippets`. Idempotent — safe to call
/// on every mutation.
///
/// Command shims take priority over snippet shims when the same name
/// appears in both (defensive — name conflicts are prevented at
/// registration time).
pub fn sync(paths: &Paths, config: &Config) -> Result<()> {
    let bin = paths.bin_dir();
    fs::create_dir_all(&bin).map_err(|source| RelayError::Io {
        path: bin.clone(),
        source,
    })?;

    // Collect unique names, preferring commands over snippets on collision.
    let mut entries: Vec<(&str, ShimKind)> = Vec::with_capacity(
        config.commands.len() + config.snippets.len(),
    );
    for name in config.commands.keys() {
        entries.push((name.as_str(), ShimKind::Command));
    }
    for name in config.snippets.keys() {
        // Skip if a command already claimed this name.
        if !config.commands.contains_key(name.as_str()) {
            entries.push((name.as_str(), ShimKind::Snippet));
        }
    }

    sync_in_at(&bin, Some(paths.root()), entries)?;
    Ok(())
}

/// Reconcile shim files in `bin_dir` against the given set of `names`.
/// All names are treated as [`ShimKind::Command`] for backward
/// compatibility with existing tests.
///
/// 1. Remove every relay-owned file whose name is not in `names`.
/// 2. Write every name that doesn't have a corresponding file.
pub fn sync_in<'a, I>(bin_dir: &Path, names: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let entries: Vec<(&str, ShimKind)> = names
        .into_iter()
        .map(|n| (n, ShimKind::Command))
        .collect();
    sync_in_at(bin_dir, None, entries)
}

fn sync_in_at<'a, I>(bin_dir: &Path, root: Option<&Path>, entries: I) -> Result<()>
where
    I: IntoIterator<Item = (&'a str, ShimKind)>,
{
    let entries: Vec<(&str, ShimKind)> = entries.into_iter().collect();
    let desired: Vec<String> = entries.iter().map(|(n, _)| platform_shim_file(n)).collect();

    // Read the current set of files in the bin directory.
    let on_disk: Vec<String> = match fs::read_dir(bin_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(source) => {
            return Err(RelayError::Io {
                path: bin_dir.to_path_buf(),
                source,
            })
        }
    };

    // Remove stale shims (only relay-owned files).
    for file_name in &on_disk {
        if !desired.iter().any(|d| d == file_name) {
            let full = bin_dir.join(file_name);
            if is_relay_shim_file(&full) {
                fs::remove_file(&full).map_err(|source| RelayError::Io { path: full, source })?;
            }
        }
    }

    // Write missing shims — one file per entry.
    for (name, kind) in &entries {
        let target = bin_dir.join(platform_shim_file(name));
        if !target.exists() {
            write_shim_file(&target, name, *kind, root)?;
        }
    }
    Ok(())
}

/// Write one **command** shim file. Uses a temp-file + rename pattern to
/// avoid leaving a half-written file if the process is killed mid-write.
pub fn write(paths: &Paths, name: &str) -> Result<()> {
    write_kind(paths, name, ShimKind::Command)
}

/// Write one shim file of the given [`ShimKind`].
pub fn write_kind(paths: &Paths, name: &str, kind: ShimKind) -> Result<()> {
    let bin = paths.bin_dir();
    fs::create_dir_all(&bin).map_err(|source| RelayError::Io {
        path: bin.clone(),
        source,
    })?;
    let target = bin.join(platform_shim_file(name));
    write_shim_file(&target, name, kind, Some(paths.root()))?;
    Ok(())
}

/// Remove one shim file (both platform variants) silently if they exist.
pub fn remove(paths: &Paths, name: &str) -> Result<()> {
    // Try both extensions so that switching platforms cleans up the other.
    for ext in [shim_ext(), OTHER_EXT] {
        let target = paths.bin_dir().join(format!("{name}{ext}"));
        let _ = fs::remove_file(&target);
    }
    Ok(())
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Write the shim script into `path`. Creates parent dirs.
///
/// If `root` is provided the shim invokes `relay --root <root> run <name>`
/// (or `relay --root <root> snippet run <name>` for snippets), so a shim is
/// always bound to the relay root that created it. This makes `--root` (and
/// `RELAY_ROOT=...`) work transparently for test sandboxes.
fn write_shim_file(path: &Path, name: &str, kind: ShimKind, root: Option<&Path>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| RelayError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    // Embed the absolute path to the running relay binary so shims do not
    // depend on `relay` being on $PATH at run time.
    let relay_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "relay".to_string());

    // Only embed --root when it differs from the default discovery path —
    // production shims should "just work" if the user moves `~`.
    let root_arg = match root {
        Some(r) => format!(" --root \"{}\"", r.display()),
        None => String::new(),
    };

    let relay_cmd = match kind {
        ShimKind::Command => format!("run {name}"),
        ShimKind::Snippet => format!("snippet run {name}"),
    };

    let tmp = path.with_file_name(format!(".{}.tmp", name));
    {
        let mut f = fs::File::create(&tmp).map_err(|source| RelayError::Io {
            path: tmp.clone(),
            source,
        })?;

        #[cfg(not(windows))]
        let res = write!(
            f,
            "#!/bin/sh\n{MAGIC}\nexec \"{relay_exe}\"{root_arg} {relay_cmd} \"$@\"\n"
        );
        #[cfg(windows)]
        let res = write!(
            f,
            "@echo off\r\nREM relay-shim v1\r\n\"{relay_exe}\"{root_arg} {relay_cmd} %*\r\n"
        );
        res.map_err(|source| RelayError::Io {
            path: tmp.clone(),
            source,
        })?;

        f.flush().map_err(|source| RelayError::Io {
            path: tmp.clone(),
            source,
        })?;
    }

    // Atomically replace the target.
    fs::rename(&tmp, path).map_err(|source| RelayError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    // Set executable bit on unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|source| {
            RelayError::Io {
                path: path.to_path_buf(),
                source,
            }
        })?;
    }

    Ok(())
}

/// Return `true` if the file at `path` is a relay-owned shim. Looks for the
/// magic marker anywhere in the first few lines so the check survives shim
/// formats that put `#!/bin/sh` or `@echo off` on line 1.
fn is_relay_shim_file(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.lines().take(3).any(|l| l.contains(MAGIC)))
        .unwrap_or(false)
}

/// Platform-native shim file name (e.g. `v`, `v.cmd`).
fn platform_shim_file(name: &str) -> String {
    format!("{name}{}", shim_ext())
}

#[cfg(not(windows))]
fn shim_ext() -> &'static str {
    ""
}

#[cfg(windows)]
fn shim_ext() -> &'static str {
    ".cmd"
}

/// The opposite extension, for cleanup cross-platform.
#[cfg(not(windows))]
const OTHER_EXT: &str = ".cmd";

#[cfg(windows)]
const OTHER_EXT: &str = "";
