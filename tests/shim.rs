//! Shim module unit tests, especially [`relay::shim::sync_in`] which
//! doesn't need `Paths` or a full config — just a `TempDir`.

use std::fs;
use std::io::Write;
use std::path::Path;

use relay::config::Paths;
use relay::shim;
use tempfile::TempDir;

/// Helper to create a bin directory and return its path.
fn bin_dir() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("temp dir");
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    (tmp, bin)
}

use std::path::PathBuf;

#[test]
fn sync_in_writes_shims_for_names() {
    let (_tmp, bin) = bin_dir();
    shim::sync_in(&bin, ["v", "g", "p"].iter().copied()).unwrap();

    for name in &["v", "g", "p"] {
        let path = shim_path(&bin, name);
        assert!(
            path.exists(),
            "shim {name} should exist at {}",
            path.display()
        );
    }
}

#[test]
fn sync_in_removes_stale_shims() {
    let (_tmp, bin) = bin_dir();

    // Write a stale shim for "x".
    let stale = shim_path(&bin, "x");
    write_shim(&stale).unwrap();
    assert!(stale.exists());

    // Sync — "x" is not in the desired set.
    shim::sync_in(&bin, ["v"].iter().copied()).unwrap();
    assert!(!stale.exists(), "stale shim should be removed");
}

#[test]
fn sync_in_does_not_remove_hand_written_files() {
    let (_tmp, bin) = bin_dir();

    // Write a file that is NOT a relay shim (no magic marker).
    let manual = bin.join("manual");
    {
        let mut f = fs::File::create(&manual).unwrap();
        writeln!(f, "echo hello").unwrap();
    }
    assert!(manual.exists());

    shim::sync_in(&bin, std::iter::empty::<&str>()).unwrap();
    assert!(
        manual.exists(),
        "hand-written file without magic marker should be preserved"
    );
}

#[test]
fn sync_in_creates_missing_and_removes_extra() {
    let (_tmp, bin) = bin_dir();

    // Old shim that should go away.
    let old = shim_path(&bin, "old");
    write_shim(&old).unwrap();

    // New name that needs a shim.
    shim::sync_in(&bin, ["new"].iter().copied()).unwrap();

    let new_file = shim_path(&bin, "new");
    assert!(new_file.exists(), "new shim should exist");
    assert!(!old.exists(), "old shim should have been removed");
}

#[test]
fn shim_content_includes_magic_and_correct_command_name() {
    let (_tmp, bin) = bin_dir();
    shim::sync_in(&bin, ["mycmd"].iter().copied()).unwrap();

    let path = shim_path(&bin, "mycmd");
    let content = fs::read_to_string(&path).unwrap();

    #[cfg(windows)]
    {
        assert!(content.contains("REM relay-shim v1"));
        assert!(content.contains("run mycmd %*"));
    }
    #[cfg(not(windows))]
    {
        assert!(content.contains("# relay-shim v1"));
        assert!(content.contains("run mycmd"));
    }
}

#[test]
fn write_and_remove_roundtrip() {
    let tmp = TempDir::new().expect("temp dir");
    let paths = Paths::at(tmp.path().join(".relay"));

    shim::write(&paths, "t").unwrap();
    let p = shim_path(&paths.bin_dir(), "t");
    assert!(p.exists(), "shim should exist after write");

    shim::remove(&paths, "t").unwrap();
    assert!(!p.exists(), "shim should be gone after remove");
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Return the platform-appropriate shim path for `name` inside `bin_dir`.
fn shim_path(bin_dir: &Path, name: &str) -> PathBuf {
    #[cfg(windows)]
    {
        bin_dir.join(format!("{name}.cmd"))
    }
    #[cfg(not(windows))]
    {
        bin_dir.join(name)
    }
}

/// Write a minimal shim file that includes the relay magic marker.
fn write_shim(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    #[cfg(not(windows))]
    writeln!(f, "#!/bin/sh")?;
    writeln!(f, "{}", relay_magic())?;
    Ok(())
}

fn relay_magic() -> &'static str {
    #[cfg(windows)]
    {
        "REM relay-shim v1"
    }
    #[cfg(not(windows))]
    {
        "# relay-shim v1"
    }
}
