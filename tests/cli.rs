//! Smoke-level integration tests via `assert_cmd`. These exercise argv
//! parsing and the cli surface; they do *not* mutate the real `~/.relay`.

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
    // We don't assert success here — `list` will try to load the real config
    // directory. We only assert that clap accepted the subcommand.
    relay().args(["list", "--help"]).assert().success();
}

#[test]
fn rejects_unknown_subcommand() {
    relay()
        .arg("nope-this-is-not-a-subcommand")
        .assert()
        .failure();
}
