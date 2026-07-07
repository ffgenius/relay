//! CLI surface. clap parses argv into a [`Cli`], and [`run`] dispatches each
//! subcommand to its module.

use clap::{Parser, Subcommand};

use crate::{
    config::{self, Paths},
    discover, doctor, registry, runner, shim, snippet, sync, ui, Result,
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
        /// Force prefix mode — extra args are always appended at runtime,
        /// even when fixed arguments are supplied.
        #[arg(long, short)]
        prefix: bool,
        /// Optional fixed arguments — supplying any without `--prefix` makes
        /// this an `exact` command.
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
        /// Force prefix mode — extra args are always appended at runtime,
        /// even when fixed arguments are supplied.
        #[arg(long, short)]
        prefix: bool,
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
    Rebuild,

    /// Remove every registered alias. Asks for confirmation unless `--yes`.
    #[command(visible_aliases = ["cls"])]
    Clear {
        /// Skip the confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },

    /// Execute a registered command. Invoked by the generated shims.
    #[command(hide = true, disable_help_flag = true, disable_version_flag = true)]
    Run {
        name: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Print the current config to stdout (yaml).
    Export {
        /// Write to a file instead of stdout.
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Merge a config file into the user's config.
    Import {
        /// Path to the YAML config file to import.
        file: std::path::PathBuf,
        /// Overwrite existing aliases on conflict. Default is to keep local.
        #[arg(long)]
        overwrite: bool,
        /// Allow importing snippets from the config file.
        #[arg(long)]
        allow_snippet: bool,
    },

    /// Sync config to a private GitHub Gist (requires `gh` CLI).
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },

    /// Manage shell code snippets with cross-shell translation.
    ///
    /// Snippets are arbitrary shell code fragments stored alongside
    /// your command aliases. Unlike regular commands (which bypass the
    /// shell), snippets are executed through a shell interpreter and
    /// benefit from automatic cross-shell translation via polysh.
    Snippet {
        #[command(subcommand)]
        action: SnippetAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum SyncAction {
    /// Create a new private Gist and link this machine to it.
    Init,
    /// Upload the local config to the linked Gist.
    Push {
        /// Exclude snippets from the push.
        #[arg(long)]
        no_snippet: bool,
    },
    /// Download the Gist config and overwrite the local config.
    Pull {
        /// Allow importing snippets from the remote Gist.
        #[arg(long)]
        allow_snippet: bool,
    },
    /// Show whether sync is configured and clean/dirty.
    Status,
    /// Link this machine to an existing Gist by ID.
    Link {
        /// The Gist ID (the hex string in the URL).
        gist_id: String,
    },
    /// Forget the linked Gist on this machine. Does not delete the Gist
    /// itself — it stays on GitHub, ready to be re-linked later.
    Unlink,
}

#[derive(Debug, Subcommand)]
pub enum SnippetAction {
    /// Create a new snippet. Content is auto-detected for shell dialect.
    ///
    /// Put --shell and --desc **before** the trailing content so clap can
    /// parse them as named flags; otherwise `trailing_var_arg` would swallow
    /// them into the content vector.
    Add {
        /// The short name for this snippet (e.g. `goback`).
        name: String,
        /// Manually specify the shell dialect (unix, powershell, cmd).
        #[arg(long)]
        shell: Option<String>,
        /// Optional human-readable description.
        #[arg(long)]
        desc: Option<String>,
        /// The shell code content.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        content: Vec<String>,
    },
    /// Delete a snippet.
    #[command(visible_aliases = ["rm"])]
    Remove {
        name: String,
        /// Skip the confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// List all registered snippets.
    #[command(visible_aliases = ["ls"])]
    List,
    /// Show the full details of one snippet.
    Info { name: String },
    /// Edit a snippet's content, description, or shell dialect.
    Edit {
        name: String,
        /// New content for the snippet.
        #[arg(long)]
        content: Option<String>,
        /// New description (pass empty string to clear).
        #[arg(long)]
        desc: Option<String>,
        /// New shell dialect.
        #[arg(long)]
        shell: Option<String>,
    },
    /// Execute a snippet, translating it to the current shell if needed.
    ///
    /// Use `{{0}}`, `{{1}}`, … placeholders in the snippet content and pass
    /// trailing arguments to substitute them at runtime:
    ///   relay snippet run killport 4000
    Run {
        name: String,
        /// Print the translated command without executing it.
        #[arg(long)]
        dry_run: bool,
        /// Skip cross-shell translation, run as-is.
        #[arg(long)]
        no_translate: bool,
        /// Run best-effort translation even when polysh cannot fully translate
        /// all segments. Without this flag, incomplete translations are
        /// rejected as an error.
        #[arg(long)]
        force: bool,
        /// Force a target shell dialect for translation (unix, powershell, cmd).
        /// Overrides auto-detection — useful for testing cross-shell results.
        #[arg(long)]
        target: Option<String>,
        /// Arguments to substitute for `{{0}}`, `{{1}}`, … placeholders in
        /// the snippet content.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Remove all snippets.
    Clear {
        /// Skip the confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
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
            prefix,
            args,
        } => registry::add(&paths, name, program, args, *prefix)?,
        Command::Remove { name } => registry::remove(&paths, name)?,
        Command::Update {
            name,
            program,
            prefix,
            args,
        } => registry::update(&paths, name, program, args, *prefix)?,
        Command::List => registry::list(&paths)?,
        Command::Info { name } => registry::info(&paths, name)?,
        Command::Discover { program } => discover::run(&paths, program.as_deref())?,
        Command::Doctor { fix } => doctor::run(&paths, *fix)?,
        Command::Rebuild => {
            let config = config::load(&paths)?;
            shim::sync(&paths, &config)?;
            ui::ok("shims regenerated");
        }
        Command::Clear { yes } => registry::clear(&paths, *yes)?,
        Command::Run { name, args } => runner::run(&paths, name, args)?,
        Command::Export { output } => registry::export(&paths, output.as_deref(), false)?,
        Command::Import { file, overwrite, allow_snippet } => registry::import(&paths, file, *overwrite, *allow_snippet)?,
        Command::Sync { action } => match action {
            SyncAction::Init => sync::init(&paths)?,
            SyncAction::Push { no_snippet } => sync::push(&paths, *no_snippet)?,
            SyncAction::Pull { allow_snippet } => sync::pull(&paths, *allow_snippet)?,
            SyncAction::Status => sync::status(&paths)?,
            SyncAction::Link { gist_id } => sync::link(&paths, gist_id)?,
            SyncAction::Unlink => sync::unlink(&paths)?,
        },
        Command::Snippet { action } => match action {
            SnippetAction::Add {
                name,
                content,
                shell,
                desc,
            } => snippet::add(&paths, name, content, shell.as_deref(), desc.as_deref())?,
            SnippetAction::Remove { name, yes } => snippet::remove(&paths, name, *yes)?,
            SnippetAction::List => snippet::list(&paths)?,
            SnippetAction::Info { name } => snippet::info(&paths, name)?,
            SnippetAction::Edit {
                name,
                content,
                desc,
                shell,
            } => snippet::edit(
                &paths,
                name,
                content.as_deref(),
                desc.as_deref(),
                shell.as_deref(),
            )?,
            SnippetAction::Run {
                name,
                dry_run,
                no_translate,
                force,
                target,
                args,
            } => snippet::run(&paths, name, *dry_run, *no_translate, target.as_deref(), *force, args)?,
            SnippetAction::Clear { yes } => snippet::clear(&paths, *yes)?,
        },
    }

    // Keep shims in sync after every mutation, but not for Run/Doctor/Export/Sync status.
    // Rebuild, Clear and sync::pull already call sync, so skip them there too.
    let should_sync = matches!(
        command,
        Command::Init
            | Command::Add { .. }
            | Command::Remove { .. }
            | Command::Update { .. }
            | Command::Import { .. }
            | Command::Snippet {
                action: SnippetAction::Add { .. }
                    | SnippetAction::Remove { .. }
                    | SnippetAction::Edit { .. }
                    | SnippetAction::Clear { .. },
            }
    );
    if should_sync {
        let config = config::load(&paths)?;
        shim::sync(&paths, &config)?;
    }

    Ok(())
}
