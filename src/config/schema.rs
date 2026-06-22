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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            commands: BTreeMap::new(),
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
