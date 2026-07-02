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
    registry::add(&paths, "c", "cargo", &[], false).unwrap();

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

    registry::add(&paths, "cb", "cargo", &["build".to_string()], false).unwrap();
    let cfg = config::load(&paths).unwrap();
    let cmd = cfg.commands.get("cb").expect("command should exist");
    assert_eq!(cmd.kind, CommandKind::Exact);
    assert_eq!(cmd.program, "cargo");
    assert_eq!(cmd.args, &["build"]);
}

#[test]
fn add_prefix_with_args() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();

    // --prefix forces Prefix mode even when fixed args are supplied.
    registry::add(&paths, "gt", "git", &["clone".to_string()], true).unwrap();
    let cfg = config::load(&paths).unwrap();
    let cmd = cfg.commands.get("gt").expect("command should exist");
    assert_eq!(cmd.kind, CommandKind::Prefix);
    assert_eq!(cmd.program, "git");
    assert_eq!(cmd.args, &["clone"]);
}

#[test]
fn add_duplicate_name_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &[], false).unwrap();

    let err = registry::add(&paths, "c", "cargo", &[], false).unwrap_err();
    assert!(matches!(err, RelayError::CommandExists(_)));
}

#[test]
fn add_empty_name_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::add(&paths, "", "cargo", &[], false).unwrap_err();
    assert!(matches!(err, RelayError::InvalidCommandName(_, _)));
}

#[test]
fn add_name_with_special_chars_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::add(&paths, "my command!", "cargo", &[], false).unwrap_err();
    assert!(matches!(err, RelayError::InvalidCommandName(_, _)));
}

#[test]
fn add_blocklisted_program_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    for bad in &["sh", "bash", "zsh", "cmd", "powershell", "pwsh"] {
        let err = registry::add(&paths, "x", bad, &[], false).unwrap_err();
        assert!(
            matches!(err, RelayError::ForbiddenProgram(_)),
            "expected forbidden for {bad}"
        );
    }
}

#[test]
fn add_strips_exe_suffix() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo.exe", &[], false).unwrap();
    let cfg = config::load(&paths).unwrap();
    assert_eq!(cfg.commands["c"].program, "cargo", ".exe suffix stripped");
}

#[test]
fn add_rejects_path_in_program() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    for bad in &["./cargo", "/usr/bin/cargo", "foo\\bar.exe"] {
        let err = registry::add(&paths, "x", bad, &[], false).unwrap_err();
        assert!(
            matches!(err, RelayError::InvalidProgram(_, _)),
            "expected InvalidProgram for {bad}, got {err:?}"
        );
    }
}

#[test]
fn remove_existing() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &[], false).unwrap();
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
    registry::add(&paths, "c", "cargo", &[], false).unwrap();
    registry::update(&paths, "c", "cargo", &["test".to_string()], false).unwrap();

    let cfg = config::load(&paths).unwrap();
    let cmd = cfg.commands.get("c").unwrap();
    assert_eq!(cmd.kind, CommandKind::Exact);
    assert_eq!(cmd.args, &["test"]);
}

#[test]
fn update_to_prefix_with_args() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "gt", "git", &["clone".to_string()], false).unwrap();
    assert_eq!(
        config::load(&paths).unwrap().commands["gt"].kind,
        CommandKind::Exact
    );

    // Update with --prefix: should flip from Exact to Prefix.
    registry::update(&paths, "gt", "git", &["clone".to_string()], true).unwrap();
    assert_eq!(
        config::load(&paths).unwrap().commands["gt"].kind,
        CommandKind::Prefix
    );
}

#[test]
fn update_nonexistent_fails() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    let err = registry::update(&paths, "nope", "cargo", &[], false).unwrap_err();
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
    registry::add(&paths, "c", "cargo", &["clippy".to_string()], false).unwrap();
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
        snippets: BTreeMap::new(),
    };
    config::save(&paths, &cfg).unwrap();

    let loaded = config::load(&paths).unwrap();
    assert_eq!(loaded.commands.len(), 1);
    assert_eq!(loaded.commands["g"].program, "git");
}

#[test]
fn clear_with_yes_removes_all() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &[], false).unwrap();
    registry::add(&paths, "cb", "cargo", &["build".to_string()], false).unwrap();
    assert_eq!(config::load(&paths).unwrap().commands.len(), 2);

    registry::clear(&paths, true).unwrap();
    assert_eq!(config::load(&paths).unwrap().commands.len(), 0);
}

#[test]
fn clear_on_empty_is_noop() {
    let (_tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    // Should not error even though there's nothing to clear.
    registry::clear(&paths, true).unwrap();
}

#[test]
fn export_appends_yaml_extension_when_missing() {
    let (tmp, paths) = tmp_paths();
    registry::init(&paths).unwrap();
    registry::add(&paths, "c", "cargo", &[], false).unwrap();

    // No extension on the export target — should become `.yaml`.
    let bare = tmp.path().join("backup");
    registry::export(&paths, Some(&bare), false).unwrap();

    assert!(!bare.exists(), "bare 'backup' should not exist");
    let yaml_path = tmp.path().join("backup.yaml");
    assert!(
        yaml_path.exists(),
        "expected backup.yaml to exist after export"
    );

    // An explicit extension should be left alone.
    let custom = tmp.path().join("custom.json");
    registry::export(&paths, Some(&custom), false).unwrap();
    assert!(custom.exists(), "custom.json should be written verbatim");
    assert!(
        !tmp.path().join("custom.json.yaml").exists(),
        ".yaml should not be appended when an extension is present"
    );
}

#[test]
fn export_and_import_roundtrip() {
    let (tmp_a, paths_a) = tmp_paths();
    let (_tmp_b, paths_b) = tmp_paths();
    registry::init(&paths_a).unwrap();
    registry::init(&paths_b).unwrap();

    // Register two commands on machine A.
    registry::add(&paths_a, "c", "cargo", &[], false).unwrap();
    registry::add(&paths_a, "cb", "cargo", &["build".to_string()], false).unwrap();

    // Export to a file in tmp_a.
    let export_file = tmp_a.path().join("export.yaml");
    registry::export(&paths_a, Some(&export_file), false).unwrap();
    assert!(export_file.exists());

    // Import into machine B (empty).
    registry::import(&paths_b, &export_file, false, false).unwrap();
    let cfg_b = config::load(&paths_b).unwrap();
    assert_eq!(cfg_b.commands.len(), 2);
    assert_eq!(cfg_b.commands["c"].program, "cargo");
    assert!(cfg_b.commands["cb"].args == vec!["build"]);
}

#[test]
fn import_default_skips_existing() {
    let (tmp_a, paths_a) = tmp_paths();
    let (_tmp_b, paths_b) = tmp_paths();
    registry::init(&paths_a).unwrap();
    registry::init(&paths_b).unwrap();

    // A has `c -> cargo`.
    registry::add(&paths_a, "c", "cargo", &[], false).unwrap();

    // B already has its own `c -> cargo build` — should be kept on import.
    registry::add(&paths_b, "c", "cargo", &["build".to_string()], false).unwrap();

    let export_file = tmp_a.path().join("export.yaml");
    registry::export(&paths_a, Some(&export_file), false).unwrap();
    registry::import(&paths_b, &export_file, false, false).unwrap();

    // B's `c` is still the exact form with args.
    let cfg_b = config::load(&paths_b).unwrap();
    assert_eq!(cfg_b.commands["c"].args, vec!["build"]);
}

#[test]
fn import_overwrite_replaces_existing() {
    let (tmp_a, paths_a) = tmp_paths();
    let (_tmp_b, paths_b) = tmp_paths();
    registry::init(&paths_a).unwrap();
    registry::init(&paths_b).unwrap();

    registry::add(&paths_a, "c", "cargo", &[], false).unwrap();
    registry::add(&paths_b, "c", "cargo", &["build".to_string()], false).unwrap();

    let export_file = tmp_a.path().join("export.yaml");
    registry::export(&paths_a, Some(&export_file), false).unwrap();
    registry::import(&paths_b, &export_file, true, false).unwrap();

    // B's `c` was overwritten with A's prefix form (no args).
    let cfg_b = config::load(&paths_b).unwrap();
    assert!(cfg_b.commands["c"].args.is_empty());
}
