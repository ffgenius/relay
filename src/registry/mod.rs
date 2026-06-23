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
            println!("[ok ]  shim dir added to PATH — open a new terminal for it to take effect");
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
    // validate_program returns the normalized form (e.g. `.exe` stripped).
    let program = validate_program(program)?;

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
            program: program.clone(),
            args: args.to_vec(),
        },
    );
    config::save(paths, &config)?;
    let suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    println!("added {name} -> {program}{suffix}");
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

/// `relay clear` — remove **every** registered alias.
///
/// Asks for interactive confirmation by default; `auto_yes = true` (from
/// `--yes`) skips the prompt for scripted use. After clearing, the central
/// dispatch path calls `shim::sync`, which will then delete every shim
/// in `~/.relay/bin/` since no commands reference them anymore.
pub fn clear(paths: &Paths, auto_yes: bool) -> Result<()> {
    let mut config = config::load(paths)?;
    let count = config.commands.len();
    if count == 0 {
        println!("(no commands registered)");
        return Ok(());
    }

    if !auto_yes {
        use std::io::{BufRead, Write};
        print!("This will remove all {count} aliases. Continue? [Y/N] ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).ok();
        let trimmed = line.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            println!("cancelled.");
            return Ok(());
        }
    }

    config.commands.clear();
    config::save(paths, &config)?;
    println!("cleared {count} alias(es)");
    Ok(())
}

/// `relay update` — replace an existing command's program/args.
pub fn update(paths: &Paths, name: &str, program: &str, args: &[String]) -> Result<()> {
    let program = validate_program(program)?;
    let mut config = config::load(paths)?;
    let entry = config
        .commands
        .get_mut(name)
        .ok_or_else(|| RelayError::UnknownCommand(name.to_string()))?;
    entry.program = program.clone();
    entry.args = args.to_vec();
    entry.kind = if args.is_empty() {
        CommandKind::Prefix
    } else {
        CommandKind::Exact
    };
    config::save(paths, &config)?;
    let suffix = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    println!("updated {name} -> {program}{suffix}");
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

/// `relay export` — write the current config as YAML.
///
/// With `output = None`, prints to stdout. With `output = Some(path)`,
/// writes to that file. If the path has no extension a `.yaml` suffix
/// is appended automatically — `relay export -o backup` writes to
/// `backup.yaml`. Existing extensions (e.g. `.yml`, `.json`) are left
/// alone so users can roll their own conventions.
///
/// The YAML format is the same as the on-disk `~/.relay/config.yaml` so
/// `relay import` on another machine can re-ingest it directly.
pub fn export(paths: &Paths, output: Option<&std::path::Path>) -> Result<()> {
    let config = config::load(paths)?;
    let yaml = serde_yaml::to_string(&config).map_err(|source| RelayError::ConfigParse {
        path: paths.config_file(),
        source,
    })?;

    match output {
        None => {
            print!("{yaml}");
        }
        Some(path) => {
            let target = ensure_yaml_extension(path);
            fs::write(&target, &yaml).map_err(|source| RelayError::Io {
                path: target.clone(),
                source,
            })?;
            println!(
                "exported {} command(s) to {}",
                config.commands.len(),
                target.display()
            );
        }
    }
    Ok(())
}

/// Append `.yaml` to `path` if it has no extension. Returns the path
/// unchanged when an extension is already present.
fn ensure_yaml_extension(path: &std::path::Path) -> std::path::PathBuf {
    if path.extension().is_none() {
        path.with_extension("yaml")
    } else {
        path.to_path_buf()
    }
}

/// `relay import <file>` — merge a YAML config from a file into the local
/// config. With `overwrite = false`, existing aliases are kept on conflict;
/// with `overwrite = true`, the imported version wins.
pub fn import(paths: &Paths, file: &std::path::Path, overwrite: bool) -> Result<()> {
    let bytes = fs::read(file).map_err(|source| RelayError::Io {
        path: file.to_path_buf(),
        source,
    })?;
    let incoming: Config =
        serde_yaml::from_slice(&bytes).map_err(|source| RelayError::ConfigParse {
            path: file.to_path_buf(),
            source,
        })?;

    let mut current = config::load(paths)?;
    let mut added = 0usize;
    let mut skipped = 0usize;
    let mut overwritten = 0usize;

    for (name, cmd) in incoming.commands {
        if current.commands.contains_key(&name) {
            if overwrite {
                current.commands.insert(name, cmd);
                overwritten += 1;
            } else {
                skipped += 1;
            }
        } else {
            current.commands.insert(name, cmd);
            added += 1;
        }
    }

    config::save(paths, &current)?;
    println!("imported: {added} added, {overwritten} overwritten, {skipped} skipped");
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

/// Validates that the program is not on the forbidden list and exists on PATH.
/// Also normalizes the name for cross-platform portability:
///
///   - Strips a trailing `.exe` suffix (Windows users sometimes type
///     `cargo.exe`; the config should store just `cargo` so it works
///     on Linux / macOS after a `relay sync pull`).
///   - Rejects program names that contain path separators (`/` or `\`).
///     Relay routes by command name, not by file path — registering
///     `/usr/bin/cargo` or `./cmd` would be a cross-platform footgun.
///
/// Exposed for reuse by the discover module.
pub(crate) fn validate_program(program: &str) -> Result<String> {
    let program = program.strip_suffix(".exe").unwrap_or(program);

    // Reject path separators — must be a bare command name on PATH.
    if program.contains('/') || program.contains('\\') {
        return Err(RelayError::InvalidProgram(
            program.to_string(),
            "use a bare command name (e.g. `cargo`), not a path",
        ));
    }

    if FORBIDDEN_PROGRAMS
        .iter()
        .any(|p| p.eq_ignore_ascii_case(program))
    {
        return Err(RelayError::ForbiddenProgram(program.to_string()));
    }
    // Principle: only register what actually exists. `which` consults PATH.
    which::which(program).map_err(|_| RelayError::ExecutableNotFound(program.to_string()))?;
    Ok(program.to_string())
}
