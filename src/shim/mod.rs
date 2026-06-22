//! Shim generation.
//!
//! Each registered command gets a tiny launcher under `~/.relay/bin/<name>`
//! that does nothing but `exec relay run <name> "$@"`. On Windows the shim is
//! a `.cmd` file; on Unix it's a `sh` script with the executable bit set.
//!
//! This module is a stub for v0.1 — `regenerate` / `clear` will be filled in
//! alongside the add/remove/update flow. `sync_marker` exists today only so
//! [`crate::cli::dispatch`] has a stable hook to call.

use crate::Result;

/// No-op hook called after every successful registry mutation.  When the real
/// shim generator lands it will be invoked from here.
pub fn sync_marker() -> Result<()> {
    Ok(())
}
