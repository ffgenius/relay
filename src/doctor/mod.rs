//! `relay doctor` — validate the environment.
//!
//! v0.1 scope (per product.md):
//!   * Relay PATH       — is `~/.relay/bin` on $PATH?
//!   * Command PATH     — is every registered program resolvable on $PATH?
//!   * Shim status      — does every registered command have a shim file?
//!   * Config integrity — does config.yaml parse cleanly?
//!
//! For now we report config + command resolvability; the PATH/shim checks
//! land with the shim generator.

use crate::{
    config::{self, Paths},
    Result,
};

pub fn run() -> Result<()> {
    let paths = Paths::discover()?;
    println!("relay root : {}", paths.root().display());
    println!("config file: {}", paths.config_file().display());
    println!("shim dir   : {}", paths.bin_dir().display());

    let config = config::load(&paths)?;
    println!("commands   : {}", config.commands.len());

    let mut missing = 0usize;
    for (name, cmd) in &config.commands {
        match which::which(&cmd.program) {
            Ok(path) => println!("  ok   {name} -> {} ({})", cmd.program, path.display()),
            Err(_) => {
                println!("  MISS {name} -> {} (not on PATH)", cmd.program);
                missing += 1;
            }
        }
    }

    if missing > 0 {
        println!("\n{missing} command(s) reference programs that aren't on PATH.");
    } else {
        println!("\nall good.");
    }
    Ok(())
}
