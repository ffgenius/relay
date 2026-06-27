//! Serializable shape of `~/.relay/config.yaml`.
//!
//! ```yaml
//! version: 1
//! commands:
//!   v:
//!     type: prefix
//!     program: vite
//!   vd:
//!     type: exact
//!     program: vite
//!     args:
//!       - dev
//! snippets:
//!   goback:
//!     type: snippet
//!     content: "cd ../"
//!     shell: unix
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Current config schema version. Bumped only on incompatible changes.
pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    /// BTreeMap so `relay list` and the yaml-on-disk are deterministically ordered.
    #[serde(default)]
    pub commands: BTreeMap<String, Command>,
    /// Snippets are shell code fragments stored alongside commands.
    #[serde(default)]
    pub snippets: BTreeMap<String, Snippet>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            commands: BTreeMap::new(),
            snippets: BTreeMap::new(),
        }
    }
}

fn default_version() -> u32 {
    CURRENT_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    #[serde(rename = "type")]
    pub kind: CommandKind,
    pub program: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandKind {
    /// User-supplied args are appended after `program`: `v dev` → `vite dev`.
    Prefix,
    /// No extra args are accepted at runtime: `vd` always runs `vite dev`.
    Exact,
}

/// A shell code snippet — arbitrary shell commands stored and synced alongside
/// command aliases. Unlike regular commands (which bypass the shell), snippets
/// are executed through a shell interpreter and support cross-shell translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    #[serde(rename = "type")]
    pub kind: SnippetKind,
    /// The shell code content (may contain pipes, redirections, etc.).
    pub content: String,
    /// The shell dialect this snippet was written in.
    pub shell: ShellDialect,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetKind {
    Snippet,
}

/// Shell dialect — mirrors polysh's [`Dialect`] with serde-friendly names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellDialect {
    Unix,
    PowerShell,
    Cmd,
}

impl From<ShellDialect> for polysh::mappings::Dialect {
    fn from(d: ShellDialect) -> Self {
        match d {
            ShellDialect::Unix => polysh::mappings::Dialect::Unix,
            ShellDialect::PowerShell => polysh::mappings::Dialect::PowerShell,
            ShellDialect::Cmd => polysh::mappings::Dialect::Cmd,
        }
    }
}

impl From<polysh::mappings::Dialect> for ShellDialect {
    fn from(d: polysh::mappings::Dialect) -> Self {
        match d {
            polysh::mappings::Dialect::Unix => ShellDialect::Unix,
            polysh::mappings::Dialect::PowerShell => ShellDialect::PowerShell,
            polysh::mappings::Dialect::Cmd => ShellDialect::Cmd,
        }
    }
}
