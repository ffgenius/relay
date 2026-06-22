//! Registry unit tests driven via [`Paths::at`] + `tempfile::TempDir`.
//!
//! These tests mutate a fake `~/.relay` inside a temporary directory and
//! never touch the user's real home directory.

use std::collections::BTreeMap;

use relay::config::{self, Command, CommandKind, Config, Paths};
use relay::registry;
use relay::RelayError;
use tempfile::TempDir;

/// Helper: create a TempDir and return (tmp, Paths).
fn tmp_paths() -> (TempDir, Paths) {
    let tmp = TempDir::new().expect("temp dir");
    let paths = Paths::at(tmp.path().join(".relay"));
    (tmp, paths)
}

#[test]
fn init_creates_dotfiles() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();

    assert!(paths.root().exists());
    assert!(paths.config_file().exists());
    // Config should be the default (empty command list).
    let cfg = config::load(&paths).unwrap();
    assert!(cfg.commands.is_empty());
}

#[test]
fn init_is_idempotent() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::init(&paths).unwrap();
    assert!(paths.root().exists());
}

#[test]
fn add_and_list_prefix_command() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();

    // We can't actually run `which("cargo")` on CI, but cargo is guaranteed
    // present in the dev environment.  Use it as the test program.
    registry::add(&paths, "c", "cargo", &[]).unwrap();

    let cfg = config::load(&paths).unwrap();
    let cmd = cfg.commands.get("c").expect("command should exist");
    assert_eq!(cmd.kind, CommandKind::Prefix);
    assert_eq!(cmd.program, "cargo");
    assert!(cmd.args.is_empty());
}

#[test]
fn add_exact_command() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();

    registry::add(&paths, "cb", "cargo", &["build".to_string()]).unwrap();
    let cfg = config::load(&paths).unwrap();
    let cmd = cfg.commands.get("cb").expect("command should exist");
    assert_eq!(cmd.kind, CommandKind::Exact);
    assert_eq!(cmd.program, "cargo");
    assert_eq!(cmd.args, &["build"]);
}

#[test]
fn add_duplicate_name_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &[]).unwrap();

    let err = registry::add(&paths, "c", "cargo", &[]).unwrap_err();
    assert!(matches!(err, RelayError::CommandExists(_)));
}

#[test]
fn add_empty_name_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::add(&paths, "", "cargo", &[]).unwrap_err();
    assert!(matches!(err, RelayError::InvalidCommandName(_, _)));
}

#[test]
fn add_name_with_special_chars_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::add(&paths, "my command!", "cargo", &[]).unwrap_err();
    assert!(matches!(err, RelayError::InvalidCommandName(_, _)));
}

#[test]
fn add_blocklisted_program_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    for bad in &["sh", "bash", "zsh", "cmd", "powershell", "pwsh"] {
        let err = registry::add(&paths, "x", bad, &[]).unwrap_err();
        assert!(
            matches!(err, RelayError::ForbiddenProgram(_)),
            "expected forbidden for {bad}"
        );
    }
}

#[test]
fn remove_existing() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &[]).unwrap();
    registry::remove(&paths, "c").unwrap();
    let cfg = config::load(&paths).unwrap();
    assert!(cfg.commands.is_empty());
}

#[test]
fn remove_nonexistent_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::remove(&paths, "nope").unwrap_err();
    assert!(matches!(err, RelayError::UnknownCommand(_)));
}

#[test]
fn update_existing() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &[]).unwrap();
    registry::update(&paths, "c", "cargo", &["test".to_string()]).unwrap();

    let cfg = config::load(&paths).unwrap();
    let cmd = cfg.commands.get("c").unwrap();
    assert_eq!(cmd.kind, CommandKind::Exact);
    assert_eq!(cmd.args, &["test"]);
}

#[test]
fn update_nonexistent_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::update(&paths, "nope", "cargo", &[]).unwrap_err();
    assert!(matches!(err, RelayError::UnknownCommand(_)));
}

#[test]
fn list_empty() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    // list doesn't return errors for empty config.
    registry::list(&paths).unwrap();
}

#[test]
fn info_returns_details() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &["clippy".to_string()]).unwrap();
    registry::info(&paths, "c").unwrap();
}

#[test]
fn info_nonexistent_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::info(&paths, "nope").unwrap_err();
    assert!(matches!(err, RelayError::UnknownCommand(_)));
}

#[test]
fn config_yaml_roundtrip() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();

    let mut cmds = BTreeMap::new();
    cmds.insert(
        "g".to_string(),
        Command {
            kind: CommandKind::Prefix,
            program: "git".to_string(),
            args: vec![],
        },
    );
    let cfg = Config {
        version: 1,
        commands: cmds,
    };
    config::save(&paths, &cfg).unwrap();

    let loaded = config::load(&paths).unwrap();
    assert_eq!(loaded.commands.len(), 1);
    assert_eq!(loaded.commands["g"].program, "git");
}
