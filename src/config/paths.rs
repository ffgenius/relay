//! Resolves the on-disk locations relay writes to.
//!
//! By spec the root is `~/.relay` on every platform (not the XDG config dir),
//! which keeps the user-visible path identical between Linux/macOS/Windows.

use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::{RelayError, Result};

/// Resolved relay paths. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// Discover paths from the real user's home directory.
    pub fn discover() -> Result<Self> {
        let base = BaseDirs::new().ok_or(RelayError::NoHomeDir)?;
        Ok(Self {
            root: base.home_dir().join(".relay"),
        })
    }

    /// Construct paths rooted at an arbitrary directory. Used by tests so they
    /// can point relay at a `tempfile::TempDir`.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// `~/.relay`
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `~/.relay/config.yaml`
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.yaml")
    }

    /// `~/.relay/bin` — where shims are written.
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }
}
