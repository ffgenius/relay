//! Runtime execution path: `relay run <name> [args...]`.
//!
//! This is the security-critical surface. It MUST NOT:
//!   * invoke any shell,
//!   * evaluate strings,
//!   * accept a program that wasn't registered.

use std::process::Command as ProcCommand;

use crate::{
    config::{self, CommandKind, Paths},
    RelayError, Result,
};

/// Execute the registered command `name`, forwarding `extra_args` to it.
///
/// For [`CommandKind::Prefix`] commands, `extra_args` is appended after the
/// stored `args`. For [`CommandKind::Exact`] commands the stored args are
/// used verbatim and any `extra_args` are rejected — that's what "exact" means.
pub fn run(paths: &Paths, name: &str, extra_args: &[String]) -> Result<()> {
    let config = config::load(paths)?;
    let cmd = config
        .commands
        .get(name)
        .ok_or_else(|| RelayError::UnknownCommand(name.to_string()))?;

    let mut argv: Vec<String> = cmd.args.clone();
    match cmd.kind {
        CommandKind::Prefix => argv.extend_from_slice(extra_args),
        CommandKind::Exact => {
            if !extra_args.is_empty() {
                return Err(RelayError::InvalidCommandName(
                    name.to_string(),
                    "exact command does not accept extra arguments",
                ));
            }
        }
    }

    // Re-resolve the executable at run time. Catches the case where the
    // program was uninstalled after registration.
    let exe = which::which(&cmd.program)
        .map_err(|_| RelayError::ExecutableNotFound(cmd.program.clone()))?;

    let status = ProcCommand::new(&exe)
        .args(&argv)
        .status()
        .map_err(|source| RelayError::Io {
            path: exe.clone(),
            source,
        })?;

    // Forward the child's exit code via process::exit so shims behave
    // transparently. `code()` returns None only on signal-terminated unix
    // processes; treat that as failure.
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
