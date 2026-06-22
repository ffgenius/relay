//! On-disk configuration: where files live, how they parse, how they save.

pub mod paths;
pub mod schema;

use std::fs;

use crate::{RelayError, Result};

pub use paths::Paths;
pub use schema::{Command, CommandKind, Config};

/// Load the user's config from `~/.relay/config.yaml`, returning an empty
/// config (not an error) when the file does not exist yet.
pub fn load(paths: &Paths) -> Result<Config> {
    let path = paths.config_file();
    if !path.exists() {
        return Ok(Config::default());
    }
    let bytes = fs::read(&path).map_err(|source| RelayError::Io {
        path: path.clone(),
        source,
    })?;
    let parsed: Config =
        serde_yaml::from_slice(&bytes).map_err(|source| RelayError::ConfigParse {
            path: path.clone(),
            source,
        })?;
    Ok(parsed)
}

/// Persist `config` to `~/.relay/config.yaml`, creating parent directories as
/// needed. Writes through a temp file would be nicer; left for a later pass.
pub fn save(paths: &Paths, config: &Config) -> Result<()> {
    let dir = paths.root();
    fs::create_dir_all(dir).map_err(|source| RelayError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let path = paths.config_file();
    let yaml = serde_yaml::to_string(config).map_err(|source| RelayError::ConfigParse {
        path: path.clone(),
        source,
    })?;
    fs::write(&path, yaml).map_err(|source| RelayError::Io {
        path,
        source,
    })?;
    Ok(())
}
