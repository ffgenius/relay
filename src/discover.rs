//! Tool discover — `relay discover [<program>]`.
//!
//! Queries the user's config and groups registered commands by the target
//! program they invoke (not by their alias name). This gives a different
//! perspective from `relay list`: instead of "what aliases do I have?" it
//! answers "what aliases do I have *for a specific tool*?".
//!
//! # Examples
//!
//! ```text
//! $ relay discover
//! vite (3 aliases):
//!   v   → vite         [Prefix]
//!   vd  → vite dev     [Exact]
//!   vb  → vite build   [Exact]
//!
//! git (2 aliases):
//!   g  → git           [Prefix]
//!   gs → git status    [Exact]
//! ```
//!
//! ```text
//! $ relay discover vite
//! vite (3 aliases):
//!   v   → vite         [Prefix]
//!   vd  → vite dev     [Exact]
//!   vb  → vite build   [Exact]
//! ```

use std::collections::BTreeMap;

use crate::config::{self, Paths};
use crate::{ui, Result};

/// Run discover with an optional program filter.
///
/// * `None` — group and print all aliases by target program.
/// * `Some(program)` — print only aliases whose target is `program`.
pub fn run(paths: &Paths, program: Option<&str>) -> Result<()> {
    let config = config::load(paths)?;
    if config.commands.is_empty() {
        ui::line("(no commands registered)");
        return Ok(());
    }

    // Build a map: program_name → [(alias, command)].
    // BTreeMap keeps programs in alphabetical order.
    let mut grouped: BTreeMap<&str, Vec<(&str, &config::Command)>> = BTreeMap::new();
    for (name, cmd) in &config.commands {
        grouped
            .entry(cmd.program.as_str())
            .or_default()
            .push((name.as_str(), cmd));
    }

    if let Some(filter) = program {
        // Single-program mode — show only that program's aliases.
        match grouped.get(filter) {
            None => {
                ui::line(format!("{filter} has no registered aliases"));
            }
            Some(entries) => {
                let count = entries.len();
                let label = if count == 1 { "alias" } else { "aliases" };
                ui::line(format!("{filter} ({count} {label}):"));
                for (alias, cmd) in entries {
                    let kind = format!("{:?}", cmd.kind);
                    ui::alias_line(alias, &cmd.program, &cmd.args, &kind);
                }
            }
        }
    } else {
        // All-programs mode — group by target program.
        let total: usize = grouped.values().map(|v| v.len()).sum();
        for (prog, entries) in &grouped {
            let label = if entries.len() == 1 {
                "alias"
            } else {
                "aliases"
            };
            ui::line(format!("{prog} ({} {label}):", entries.len()));
            for (alias, cmd) in entries {
                let kind = format!("{:?}", cmd.kind);
                ui::alias_line(alias, &cmd.program, &cmd.args, &kind);
            }
            ui::blank();
        }
        ui::line(format!("total: {total} alias(es)"));
    }

    Ok(())
}
