//! CLI surface. clap parses argv into a [`Cli`], and [`run`] dispatches each
//! subcommand to its module. Subcommands that don't have a real implementation
//! yet return [`RelayError::Unimplemented`] so the surface compiles end-to-end.

use clap::{Parser, Subcommand};

use crate::{doctor, registry, runner, shim, RelayError, Result};

#[derive(Debug, Parser)]
#[command(
    name = "relay",
    version,
    about = "Secure cross-platform command router",
    long_about = None,
)]
pub struct Cli {
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
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Remove a registered command.
    Remove {
        name: String,
    },

    /// Replace the program/args of an existing command.
    Update {
        name: String,
        program: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// List all registered commands.
    List,

    /// Show the registered details of one command.
    Info {
        name: String,
    },

    /// Validate the environment, config, and shim state.
    Doctor,

    /// Execute a registered command. Invoked by the generated shims.
    #[command(hide = true)]
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
    dispatch(cli.command)
}

/// Dispatch a parsed [`Command`]. Split from [`run`] so tests can drive it
/// without re-parsing argv.
pub fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Init => registry::init(),
        Command::Add { name, program, args } => registry::add(&name, &program, &args),
        Command::Remove { name } => registry::remove(&name),
        Command::Update { name, program, args } => registry::update(&name, &program, &args),
        Command::List => registry::list(),
        Command::Info { name } => registry::info(&name),
        Command::Doctor => doctor::run(),
        Command::Run { name, args } => runner::run(&name, &args),
        Command::Export => Err(RelayError::Unimplemented("relay export")),
        Command::Import => Err(RelayError::Unimplemented("relay import")),
    }
    .and_then(|_| {
        // Keep the shim-regeneration hook explicit so it's easy to find later.
        let _ = shim::sync_marker();
        Ok(())
    })
}
