//! Command registry — the user-facing CRUD on `~/.relay/config.yaml`.
//!
//! Handler functions take `&Paths` so tests can inject a `TempDir` instead of
//! mutating the user's real `~/.relay`. Shim regeneration is done centrally
//! from `cli::dispatch` after a successful mutation, not here.

use std::fs;

use crate::{
    config::{self, Command, CommandKind, Config, Paths},
    path_setup::{self, InstallOutcome},
    RelayError, Result,
};

/// Programs that are never allowed as a registration target — registering a
/// shell would defeat Principle 1 (Relay 不执行 Shell).
const FORBIDDEN_PROGRAMS: &[&str] = &["sh", "bash", "zsh", "cmd", "powershell", "pwsh"];

/// `relay init` — ensure `~/.relay` and an empty config exist. The shim bin
/// directory is created by the shim module's sync step that runs right after.
///
/// As a convenience for first-run UX, this also tries to put
/// `~/.relay/bin` on the user's persistent PATH. Failures here are not
/// fatal — they degrade to a printed hint.
///
/// The PATH side-effect is skipped when running against a non-default root
/// (i.e. `--root` or `RELAY_ROOT` is set) so that integration tests and
/// `--root <tmpdir>` smoke runs don't pollute the user's real environment.
pub fn init(paths: &Paths) -> Result<()> {
    fs::create_dir_all(paths.root()).map_err(|source| RelayError::Io {
        path: paths.root().to_path_buf(),
        source,
    })?;
    if !paths.config_file().exists() {
        config::save(paths, &Config::default())?;
    }
    println!("relay initialised at {}", paths.root().display());

    // Only touch the user's persistent PATH for the canonical relay root.
    // Sandboxed runs (tests / `--root`) get the shim dir created but skip
    // the system-wide PATH edit.
    if !is_default_root(paths) {
        return Ok(());
    }

    // Best-effort PATH install — never bubbles up to the caller. New shells
    // then pick `n`, `v`, etc. up automatically.
    match path_setup::install(paths) {
        InstallOutcome::AlreadyPresent => {
            println!("[ok ]  shim dir already on PATH");
        }
        InstallOutcome::Installed => {
            println!(
                "[ok ]  shim dir added to PATH — open a new terminal for it to take effect"
            );
        }
        InstallOutcome::Unsupported(reason) | InstallOutcome::Failed(reason) => {
            println!("[warn] could not auto-update PATH: {reason}");
            println!(
                "       add `{}` to your PATH manually",
                paths.bin_dir().display()
            );
        }
    }
    Ok(())
}

/// True iff `paths` resolves to the platform default `~/.relay` — used to
/// suppress destructive system-level side effects when running under a
/// `--root <tmpdir>` override.
fn is_default_root(paths: &Paths) -> bool {
    let Ok(default) = config::Paths::discover() else {
        return false;
    };
    paths.root() == default.root()
}

/// `relay add` — register a new command.
pub fn add(paths: &Paths, name: &str, program: &str, args: &[String]) -> Result<()> {
    validate_name(name)?;
    validate_program(program)?;

    let mut config = config::load(paths)?;
    if config.commands.contains_key(name) {
        return Err(RelayError::CommandExists(name.to_string()));
    }

    let kind = if args.is_empty() {
        CommandKind::Prefix
    } else {
        CommandKind::Exact
    };
    config.commands.insert(
        name.to_string(),
        Command {
            kind,
            program: program.to_string(),
            args: args.to_vec(),
        },
    );
    config::save(paths, &config)?;
    println!("added {name} -> {program}");
    Ok(())
}

/// `relay remove` — delete a registered command.
pub fn remove(paths: &Paths, name: &str) -> Result<()> {
    let mut config = config::load(paths)?;
    if config.commands.remove(name).is_none() {
        return Err(RelayError::UnknownCommand(name.to_string()));
    }
    config::save(paths, &config)?;
    println!("removed {name}");
    Ok(())
}

/// `relay update` — replace an existing command's program/args.
pub fn update(paths: &Paths, name: &str, program: &str, args: &[String]) -> Result<()> {
    validate_program(program)?;
    let mut config = config::load(paths)?;
    let entry = config
        .commands
        .get_mut(name)
        .ok_or_else(|| RelayError::UnknownCommand(name.to_string()))?;
    entry.program = program.to_string();
    entry.args = args.to_vec();
    entry.kind = if args.is_empty() {
        CommandKind::Prefix
    } else {
        CommandKind::Exact
    };
    config::save(paths, &config)?;
    println!("updated {name}");
    Ok(())
}

/// `relay list` — print all registered commands.
pub fn list(paths: &Paths) -> Result<()> {
    let config = config::load(paths)?;
    if config.commands.is_empty() {
        println!("(no commands registered — try `relay add v vite`)");
        return Ok(());
    }
    for (name, cmd) in &config.commands {
        let suffix = if cmd.args.is_empty() {
            String::new()
        } else {
            format!(" {}", cmd.args.join(" "))
        };
        println!(
            "{name:<12} [{kind:?}] {program}{suffix}",
            kind = cmd.kind,
            program = cmd.program
        );
    }
    Ok(())
}

/// `relay info <name>` — render the full details of one command.
pub fn info(paths: &Paths, name: &str) -> Result<()> {
    let config = config::load(paths)?;
    let cmd = config
        .commands
        .get(name)
        .ok_or_else(|| RelayError::UnknownCommand(name.to_string()))?;
    println!("name    : {name}");
    println!("type    : {:?}", cmd.kind);
    println!("program : {}", cmd.program);
    println!("args    : {:?}", cmd.args);
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(RelayError::InvalidCommandName(
            name.to_string(),
            "name is empty",
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(RelayError::InvalidCommandName(
            name.to_string(),
            "only ASCII letters, digits, '-' and '_' are allowed",
        ));
    }
    Ok(())
}

fn validate_program(program: &str) -> Result<()> {
    if FORBIDDEN_PROGRAMS
        .iter()
        .any(|p| p.eq_ignore_ascii_case(program))
    {
        return Err(RelayError::ForbiddenProgram(program.to_string()));
    }
    // Principle: only register what actually exists. `which` consults PATH.
    which::which(program).map_err(|_| RelayError::ExecutableNotFound(program.to_string()))?;
    Ok(())
}
