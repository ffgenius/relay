//! Ensure `~/.relay/bin` is on $PATH.
//!
//! This module implements per-platform logic for adding the relay shim
//! directory to the user's persistent PATH configuration so that shim
//! commands (e.g. `v`, `g`, `n`) are immediately available from the shell.
//!
//! # Platform strategies
//!
//! - **Windows** — writes to `HKCU\Environment\Path` via the registry,
//!   preserving the `REG_EXPAND_SZ` type so `%USERPROFILE%`-style entries
//!   continue to work.
//! - **Unix** — detects the shell from `$SHELL` and appends a marked block
//!   to the appropriate rc file (`~/.bashrc`, `~/.zshrc`, or fish's
//!   `config.fish`). Idempotent: repeated runs do not duplicate the entry.

use std::path::Path;

use crate::config::Paths;

/// Outcome of an [`install`] attempt.
pub enum InstallOutcome {
    /// The shim directory was already on PATH — nothing to do.
    AlreadyPresent,
    /// Successfully added to the profile / user environment.
    /// The change takes effect in **new** terminals.
    Installed,
    /// Could not determine which profile to write (e.g. unknown shell).
    /// The string is a human-readable reason.
    Unsupported(String),
    /// The write was attempted but failed.
    /// The string describes what went wrong.
    Failed(String),
}

/// Try to add the relay bin directory to the user's persistent PATH
/// configuration for the current platform.
///
/// Returns an [`InstallOutcome`] describing what happened. The operation is
/// idempotent — running it multiple times will not duplicate the entry.
pub fn install(paths: &Paths) -> InstallOutcome {
    install_dir(&paths.bin_dir())
}

// ─── Internal helpers ─────────────────────────────────────────────────────

fn install_dir(bin_dir: &Path) -> InstallOutcome {
    // Short-circuit: if it's already on PATH, no need to write anything.
    if path_contains_on_path(bin_dir) {
        return InstallOutcome::AlreadyPresent;
    }

    // Each platform branch is its own item so that only the active one is
    // compiled and we don't trip rust-analyzer's "statement vs. tail expr"
    // heuristic with three sibling `#[cfg]` blocks.
    #[cfg(windows)]
    return install_windows(bin_dir);

    #[cfg(unix)]
    return install_unix(bin_dir);

    #[cfg(not(any(windows, unix)))]
    return InstallOutcome::Unsupported(
        "unsupported platform — add the shim dir to your PATH manually".into(),
    );
}

/// Does `$PATH` already contain the canonical form of `dir`?
fn path_contains_on_path(dir: &Path) -> bool {
    let want = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    for entry in std::env::split_paths(&path_var) {
        let candidate = std::fs::canonicalize(&entry).unwrap_or(entry);
        if candidate == want {
            return true;
        }
    }
    false
}

// ─── Windows (registry) ──────────────────────────────────────────────────

#[cfg(windows)]
mod platform {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ};
    use winreg::RegKey;

    use super::InstallOutcome;

    /// Sub-key for the user's environment variables.
    const ENV_KEY: &str = "Environment";

    pub fn install_to_path(bin_dir: &Path) -> InstallOutcome {
        let bin_str = match bin_dir.to_str() {
            Some(s) => s,
            None => {
                return InstallOutcome::Failed("bin directory path is not valid UTF-8".into());
            }
        };

        let hkcu = match RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(ENV_KEY, KEY_READ | KEY_WRITE)
        {
            Ok(k) => k,
            Err(e) => {
                return InstallOutcome::Failed(format!("could not open HKCU\\{ENV_KEY}: {e}"));
            }
        };

        // Read the existing Path value, preserving the registry type.
        // Most Windows UI tools set this as REG_EXPAND_SZ so that
        // %USERPROFILE%-style references in PATH entries are expanded at
        // runtime. We read the raw (unexpanded) string for comparison.
        // Registry strings are UTF-16 LE, so we decode to a Rust String.
        let (current_value, reg_type) = match hkcu.get_raw_value("Path") {
            Ok(val) => {
                let s = u16_from_reg_bytes(&val.bytes);
                (s, val.vtype)
            }
            Err(_) => (String::new(), REG_EXPAND_SZ),
        };

        // Check every PATH segment (split by ';'), supporting both the
        // literal path and its canonical form so we don't double-add.
        if current_value
            .split(';')
            .any(|seg| paths_match(seg, bin_dir))
        {
            return InstallOutcome::AlreadyPresent;
        }

        // Prepend (not append). Two reasons:
        //   1. cmd.exe truncates the final %PATH% to 2047 chars at process
        //      creation time. If the user's PATH is already crowded, an
        //      appended entry vanishes silently — exactly the bug we just
        //      hit on real installs.
        //   2. Relay's namespace (`v`, `g`, `n`, ...) should win against
        //      legacy executables that happen to share a name. A `g.exe`
        //      somewhere else on PATH should not beat `relay add g git`.
        let new_value = if current_value.is_empty() {
            bin_str.to_string()
        } else {
            format!("{bin_str};{current_value}")
        };

        // Soft cap warning: total HKCU Path values much over 2KB risk
        // truncation by the time they reach a cmd window (Windows reserves
        // 2047 chars for the *combined* user+system PATH; user-only above
        // ~1900 starts crowding it). We still write it, but flag it.
        let length_warning = if new_value.len() > 1900 {
            Some(format!(
                "[warn] user PATH is {} chars long — Windows may truncate it",
                new_value.len()
            ))
        } else {
            None
        };

        // Encode as UTF-16 LE (including null terminator) and write back
        // with the *same* registry type. If it was REG_EXPAND_SZ, writing
        // as REG_SZ would break %VAR%-style entries in other segments.
        let bytes = to_utf16_le(&new_value);
        if let Err(e) = hkcu.set_raw_value(
            "Path",
            &winreg::RegValue {
                bytes: bytes.into(),
                vtype: reg_type,
            },
        ) {
            return InstallOutcome::Failed(format!(
                "could not write to HKCU\\{ENV_KEY}\\Path: {e}"
            ));
        }

        // Broadcast WM_SETTINGCHANGE so explorer / running shells re-read
        // the environment. Without this, the registry value is correct but
        // new cmd windows inherit explorer's stale cached PATH until the
        // user logs out — confusing, since `relay init` prints "added to
        // PATH" but `n ls` still fails.
        broadcast_environment_change();

        if let Some(w) = length_warning {
            println!("{w}");
        }

        InstallOutcome::Installed
    }

    /// Send `WM_SETTINGCHANGE` to all top-level windows with `lParam`
    /// pointing at the string "Environment". Explorer listens for this and
    /// reloads its env block; any process spawned after this point inherits
    /// the new PATH. Best-effort — if the broadcast fails the user just
    /// needs to log out / log back in, which is what they would have had
    /// to do anyway.
    fn broadcast_environment_change() {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        // SAFETY: SendMessageTimeoutW is a system call with stable ABI;
        // we pass a properly null-terminated UTF-16 buffer for lParam.
        // The HWND_BROADCAST + SMTO_ABORTIFHUNG + 5s timeout combo is the
        // documented pattern for "tell everyone, but don't wedge if a
        // hung window doesn't reply".
        #[link(name = "user32")]
        extern "system" {
            fn SendMessageTimeoutW(
                hwnd: isize,
                msg: u32,
                wparam: usize,
                lparam: *const u16,
                fuflags: u32,
                timeout: u32,
                result: *mut usize,
            ) -> isize;
        }
        const HWND_BROADCAST: isize = 0xFFFF;
        const WM_SETTINGCHANGE: u32 = 0x001A;
        const SMTO_ABORTIFHUNG: u32 = 0x0002;

        let param: Vec<u16> = OsStr::new("Environment")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut result: usize = 0;
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                param.as_ptr(),
                SMTO_ABORTIFHUNG,
                5_000,
                &mut result,
            );
        }
    }

    /// Decode a UTF-16 LE byte slice (with or without trailing NUL) into a
    /// Rust `String`. Any unpaired surrogates are replaced with U+FFFD.
    fn u16_from_reg_bytes(bytes: &[u8]) -> String {
        let u16_words: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&w| w != 0) // stop at NUL terminator
            .collect();
        String::from_utf16_lossy(&u16_words)
    }

    /// Encode a UTF-8 `&str` as a UTF-16 LE byte vector, **with** a trailing
    /// NUL (Windows registry convention for `REG_SZ` / `REG_EXPAND_SZ`).
    fn to_utf16_le(s: &str) -> Vec<u8> {
        let u16_words: Vec<u16> = OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut bytes = Vec::with_capacity(u16_words.len() * 2);
        for w in u16_words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        bytes
    }

    /// Compare a PATH segment against `bin_dir`, trying both literal and
    /// canonical comparisons.
    fn paths_match(segment: &str, bin_dir: &Path) -> bool {
        let seg_path = std::path::Path::new(segment);
        // Literal match (fast path)
        if seg_path == bin_dir {
            return true;
        }
        // Canonicalised match (handles symlinks, case differences on Win)
        let seg_canon = std::fs::canonicalize(seg_path).ok();
        let dir_canon = std::fs::canonicalize(bin_dir).ok();
        if let (Some(a), Some(b)) = (seg_canon, dir_canon) {
            // Case-insensitive comparison on Windows
            return a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase();
        }
        false
    }

    // Test-only re-exports of the internal helpers so the parent module's
    // unit tests can call them without touching the registry.
    #[cfg(test)]
    pub(super) fn __test_to_utf16_le(s: &str) -> Vec<u8> {
        to_utf16_le(s)
    }

    #[cfg(test)]
    pub(super) fn __test_u16_from_reg_bytes(bytes: &[u8]) -> String {
        u16_from_reg_bytes(bytes)
    }
}

#[cfg(windows)]
fn install_windows(bin_dir: &Path) -> InstallOutcome {
    platform::install_to_path(bin_dir)
}

// ─── Unix (shell rc files) ───────────────────────────────────────────────

#[cfg(unix)]
mod platform {
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;

    use super::InstallOutcome;

    /// Marker comments that bracket the relay PATH block in rc files.
    const MARKER_START: &str = "# >>> relay shim path >>>";
    const MARKER_END: &str = "# <<< relay shim path <<<";

    /// Path entries to add per shell type.
    const BASH_EXPORT: &str = "export PATH=\"$HOME/.relay/bin:$PATH\"";
    const ZSH_EXPORT: &str = "export PATH=\"$HOME/.relay/bin:$PATH\"";
    const FISH_EXPORT: &str = "fish_add_path \"$HOME/.relay/bin\"";

    /// Attempt to add the relay bin directory to the user's shell profile.
    pub fn install_to_profile() -> InstallOutcome {
        let shell = match std::env::var("SHELL") {
            Ok(s) => s,
            Err(_) => {
                return InstallOutcome::Unsupported(
                    "$SHELL is not set — manually add the shim dir to your PATH".into(),
                );
            }
        };

        // Determine the rc file path and the line to add.
        let (rc_path, export_line) = match shell_path(&shell) {
            Some(pair) => pair,
            None => {
                return InstallOutcome::Unsupported(format!(
                    "unknown shell '{shell}' — \
                     manually add ~/.relay/bin to your PATH"
                ));
            }
        };

        // If the rc file doesn't exist yet, create it.
        let parent = rc_path.parent().unwrap();
        if let Err(e) = fs::create_dir_all(parent) {
            return InstallOutcome::Failed(format!("could not create {}: {e}", parent.display()));
        }

        // Read existing content (if any) to check for our marker block.
        let existing = match fs::read_to_string(&rc_path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                return InstallOutcome::Failed(format!(
                    "could not read {}: {e}",
                    rc_path.display()
                ));
            }
        };

        if existing.contains(MARKER_START) && existing.contains(MARKER_END) {
            // Block already exists — nothing to do.
            return InstallOutcome::AlreadyPresent;
        }

        // Append the block.
        let block = format!("\n{MARKER_START}\n{export_line}\n{MARKER_END}\n");
        let mut file = match fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&rc_path)
        {
            Ok(f) => f,
            Err(e) => {
                return InstallOutcome::Failed(format!(
                    "could not open {} for writing: {e}",
                    rc_path.display()
                ));
            }
        };
        if let Err(e) = file.write_all(block.as_bytes()) {
            return InstallOutcome::Failed(format!(
                "could not write to {}: {e}",
                rc_path.display()
            ));
        }
        if let Err(e) = file.flush() {
            return InstallOutcome::Failed(format!("could not flush {}: {e}", rc_path.display()));
        }

        InstallOutcome::Installed
    }

    /// Given a full path like `/bin/bash` or `/usr/bin/zsh`, return the
    /// rc file path and the export line appropriate for that shell.
    fn shell_path(shell: &str) -> Option<(PathBuf, &'static str)> {
        let name = std::path::Path::new(shell)
            .file_name()
            .and_then(|n| n.to_str())?;

        let home = std::env::var("HOME").ok()?;

        match name {
            "bash" => Some((PathBuf::from(&home).join(".bashrc"), BASH_EXPORT)),
            "zsh" => Some((PathBuf::from(&home).join(".zshrc"), ZSH_EXPORT)),
            "fish" => Some((
                PathBuf::from(&home).join(".config/fish/config.fish"),
                FISH_EXPORT,
            )),
            "sh" => Some((PathBuf::from(&home).join(".profile"), BASH_EXPORT)),
            _ => None,
        }
    }
}

#[cfg(unix)]
fn install_unix(_bin_dir: &Path) -> InstallOutcome {
    platform::install_to_profile()
}

#[cfg(all(test, windows))]
mod tests {
    use super::platform;

    /// Verify the UTF-16 LE encode/decode pair round-trips ASCII PATH
    /// values and trims the trailing NUL produced by `to_utf16_le`.
    /// Re-exported via a `#[cfg(test)] pub(super)` wrapper in `platform`.
    #[test]
    fn utf16_round_trip_preserves_ascii() {
        let original = r"C:\Users\Foo\.relay\bin;%SystemRoot%\System32";
        let bytes = platform::__test_to_utf16_le(original);
        // First 2 bytes encode the first char 'C' (0x43, 0x00 in LE).
        assert_eq!(&bytes[0..2], &[0x43, 0x00]);
        // Trailing NUL terminator: last two bytes are 0x00 0x00.
        assert_eq!(&bytes[bytes.len() - 2..], &[0x00, 0x00]);
        let decoded = platform::__test_u16_from_reg_bytes(&bytes);
        assert_eq!(decoded, original);
    }

    /// Verify that ill-formed (odd-length) byte slices don't panic — we
    /// strip the trailing odd byte rather than crash.
    #[test]
    fn utf16_decode_tolerates_odd_length() {
        let bytes = [0x41, 0x00, 0x42, 0x00, 0xFF];
        let decoded = platform::__test_u16_from_reg_bytes(&bytes);
        assert_eq!(decoded, "AB");
    }

    /// A path containing non-ASCII characters survives the round-trip.
    #[test]
    fn utf16_round_trip_preserves_unicode() {
        let original = r"C:\Users\日本語\.relay\bin";
        let bytes = platform::__test_to_utf16_le(original);
        let decoded = platform::__test_u16_from_reg_bytes(&bytes);
        assert_eq!(decoded, original);
    }
}
