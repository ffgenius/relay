use std::path::PathBuf;

use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, RelayError>;

/// Top-level error type. Each variant carries enough context to render a
/// useful single-line message via `Display`, and forwards underlying I/O or
/// parse errors through `source()` for the binary's cause-chain printout.
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("feature not yet implemented: {0}")]
    Unimplemented(&'static str),

    #[error("command `{0}` is not registered")]
    UnknownCommand(String),

    #[error("command `{0}` is already registered")]
    CommandExists(String),

    #[error("`{0}` is on the blocklist of forbidden programs")]
    ForbiddenProgram(String),

    #[error("executable `{0}` was not found on PATH")]
    ExecutableNotFound(String),

    #[error("invalid command name `{0}`: {reason}", reason = .1)]
    InvalidCommandName(String, &'static str),

    #[error("invalid program `{0}`: {reason}", reason = .1)]
    InvalidProgram(String, &'static str),

    #[error("could not determine the user's home directory")]
    NoHomeDir,

    #[error("config file at {path} is malformed")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("i/o error at {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
