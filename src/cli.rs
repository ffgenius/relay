//! CLI surface. clap parses argv into a [`Cli`], and [`run`] dispatches each
//! subcommand to its module.

use clap::{Parser, Subcommand};

use crate::{
    config::{self, Paths},
    discover, doctor, registry, runner, shim, Result,
};

#[derive(Debug, Parser)]
#[command(
    name = "relay",
    version,
    about = "Secure cross-platform command router",
    long_about = None,
)]
pub struct Cli {
    /// Override the relay root directory. Defaults to `~/.relay`.
    /// Mainly useful for tests and for trying relay without touching $HOME.
    #[arg(long, global = true, env = "RELAY_ROOT")]
    pub root: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create the relay config directory and shim folder.
    Init,

    /// Register a new command.
    ///
    /// Prefix form:  `relay add v vite`
    /// Exact  form:  `relay add vd vite dev`
    Add {
        /// The short name the user will type (e.g. `v`).
        name: String,
        /// The target program (e.g. `vite`).
        program: String,
        /// Optional fixed arguments — supplying any makes this an `exact` command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Remove a registered command.
    #[command(visible_aliases = ["rm"])]
    Remove { name: String },

    /// Replace the program/args of an existing command.
    Update {
        name: String,
        program: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List all registered commands.
    #[command(visible_aliases = ["ls"])]
    List,

    /// Show the registered details of one command.
    Info { name: String },

    /// Group registered aliases by their target program.
    ///
    /// Without arguments, lists every program along with its aliases.
    /// With a program name, shows only that program's aliases.
    Discover {
        /// Filter by target program (e.g. `vite`, `git`).
        program: Option<String>,
    },

    /// Validate the environment, config, and shim state.
    Doctor {
        /// Automatically fix shim inconsistencies (re-generate missing shims,
        /// remove orphaned ones).
        #[arg(long)]
        fix: bool,
    },

    /// Regenerate all shims from the current config. Shims are normally kept
    /// in sync automatically — this is an explicit bulk-reset command.
    RebuildShims,

    /// Execute a registered command. Invoked by the generated shims.
    #[command(hide = true, disable_help_flag = true, disable_version_flag = true)]
    Run {
        name: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Print the current config to stdout (yaml).
    Export,

    /// Read a config from stdin and merge it into the user's config.
    Import,
}

/// Entry point used by `main.rs` and by integration tests.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    dispatch_with_root(cli.command, cli.root)
}

/// Dispatch a parsed [`Command`]. Split from [`run`] so tests can drive it
/// without re-parsing argv.
pub fn dispatch(command: Command) -> Result<()> {
    dispatch_with_root(command, None)
}

fn dispatch_with_root(command: Command, root: Option<std::path::PathBuf>) -> Result<()> {
    // Resolve paths once so modules don't call `Paths::discover()` themselves.
    let paths = match root {
        Some(r) => Paths::at(r),
        None => Paths::discover()?,
    };

    match &command {
        Command::Init => registry::init(&paths)?,
        Command::Add {
            name,
            program,
            args,
        } => registry::add(&paths, name, program, args)?,
        Command::Remove { name } => registry::remove(&paths, name)?,
        Command::Update {
            name,
            program,
            args,
        } => registry::update(&paths, name, program, args)?,
        Command::List => registry::list(&paths)?,
        Command::Info { name } => registry::info(&paths, name)?,
        Command::Discover { program } => discover::run(&paths, program.as_deref())?,
        Command::Doctor { fix } => doctor::run(&paths, *fix)?,
        Command::RebuildShims => {
            let config = config::load(&paths)?;
            shim::sync(&paths, &config)?;
            println!("shims regenerated");
        }
        Command::Run { name, args } => runner::run(&paths, name, args)?,
        Command::Export => return Err(crate::RelayError::Unimplemented("relay export")),
        Command::Import => return Err(crate::RelayError::Unimplemented("relay import")),
    }

    // Keep shims in sync after every mutation, but not for Run/Doctor/Export/Import.
    // RebuildShims already calls sync, so skip it there too.
    let should_sync = matches!(
        command,
        Command::Init | Command::Add { .. } | Command::Remove { .. } | Command::Update { .. }
    );
    if should_sync {
        let config = config::load(&paths)?;
        shim::sync(&paths, &config)?;
    }

    Ok(())
}
