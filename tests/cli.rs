//! Smoke-level integration tests via `assert_cmd`. These exercise argv
//! parsing and the cli surface.

use assert_cmd::Command;
use predicates::str::contains;

fn relay() -> Command {
    Command::cargo_bin("relay").expect("binary built")
}

#[test]
fn prints_help() {
    relay()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Secure cross-platform command router"));
}

#[test]
fn list_subcommand_is_known() {
    relay().args(["list", "--help"]).assert().success();
}

#[test]
fn rejects_unknown_subcommand() {
    relay()
        .arg("nope-this-is-not-a-subcommand")
        .assert()
        .failure();
}

#[test]
fn doctor_subcommand_is_known() {
    // `doctor` always tries to read the real config; we only test clap
    // recognised it (don't care about exit code).
    relay().args(["doctor", "--help"]).assert().success();
}

#[test]
fn rebuild_shims_subcommand_is_known() {
    relay().args(["rebuild-shims", "--help"]).assert().success();
}
