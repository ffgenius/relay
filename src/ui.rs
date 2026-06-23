//! Output styling — all printing should go through this module.
//!
//! Centralising styling here keeps the rest of the codebase free of
//! direct `owo_colors` / `indicatif` calls and gives us one place to
//! switch backends, suppress colour for `NO_COLOR` users, or pivot to
//! structured (JSON) output in a future `--quiet` / `--json` mode.
//!
//! # When to use which
//!
//! - [`ok`] / [`warn`] / [`err`] / [`note`] for status lines
//! - [`header`] for section titles (`relay doctor`'s "relay root: ..." block)
//! - [`field`] for `key : value` rows
//! - [`alias_line`] for the standard "n  → nvm ls  [Exact]" alias rendering
//!   used by `list`, `info`, and `discover`
//! - [`spinner`] when wrapping a long-running operation (sync push/pull)
//!
//! All of these respect `NO_COLOR` and pipe detection automatically — when
//! stdout is not a TTY, ANSI escapes are stripped (owo-colors' `if_supports_color`
//! handles that for us).

use std::fmt::Display;
use std::io::IsTerminal;

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::{OwoColorize, Stream};

/// Print a green `[ok ]` followed by `msg`.
pub fn ok(msg: impl Display) {
    println!("{} {msg}", "[ok ]".if_supports_color(Stream::Stdout, |t| t.green()));
}

/// Print a yellow `[warn]` followed by `msg`.
pub fn warn(msg: impl Display) {
    println!("{} {msg}", "[warn]".if_supports_color(Stream::Stdout, |t| t.yellow()));
}

/// Print a red `[err ]` followed by `msg` to **stderr**.
pub fn err(msg: impl Display) {
    eprintln!("{} {msg}", "[err ]".if_supports_color(Stream::Stderr, |t| t.red()));
}

/// A neutral note — useful for "Open a new terminal" / "Re-link any time".
/// Renders as cyan `[note]` so it's distinct from warnings without
/// implying anything is wrong.
pub fn note(msg: impl Display) {
    println!("{} {msg}", "[note]".if_supports_color(Stream::Stdout, |t| t.cyan()));
}

/// Bold section header. Used for "relay root: ..." blocks at the top of
/// `relay doctor`, `relay sync status` etc.
pub fn header(label: impl Display, value: impl Display) {
    println!(
        "{} {value}",
        format!("{label}").if_supports_color(Stream::Stdout, |t| t.bold())
    );
}

/// `key : value` row. The key is dimmed so values stand out.
pub fn field(key: &str, value: impl Display) {
    println!(
        "{:<8} : {value}",
        key.if_supports_color(Stream::Stdout, |t| t.dimmed())
    );
}

/// Render one alias row the way `relay list` / `discover` / `info` want it.
///
/// `name` is bold, the arrow is dimmed, `program` is plain, baked args are
/// dim, and `[Kind]` is dimmed + italic. Width-aligned so multi-line tables
/// still line up.
pub fn alias_line(name: &str, program: &str, args: &[String], kind_label: &str) {
    let args_part = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };
    println!(
        "  {name:<12} {arrow} {program}{args_dim}  {kind}",
        name = name.if_supports_color(Stream::Stdout, |t| t.bold()),
        arrow = "→".if_supports_color(Stream::Stdout, |t| t.dimmed()),
        program = program,
        args_dim = args_part.if_supports_color(Stream::Stdout, |t| t.dimmed()),
        kind = format!("[{kind_label}]").if_supports_color(Stream::Stdout, |t| t.dimmed()),
    );
}

/// Plain-text message — pass through to `println!` but kept here so we
/// can later add `--quiet` suppression in one place if we ever need it.
pub fn line(msg: impl Display) {
    println!("{msg}");
}

/// Empty line — exists so callers don't sprinkle `println!()` directly.
pub fn blank() {
    println!();
}

/// Create a spinner for a long-running operation. Returns the
/// `ProgressBar` so the caller can `finish_with_message` once done.
///
/// When stdout is not a terminal (e.g. piped to a file or running under
/// CI), the spinner is automatically hidden — only the final message
/// will appear. This keeps logs clean and avoids ANSI noise in captured
/// output.
pub fn spinner(msg: &'static str) -> ProgressBar {
    let pb = if std::io::stdout().is_terminal() {
        let p = ProgressBar::new_spinner();
        p.enable_steady_tick(std::time::Duration::from_millis(100));
        p
    } else {
        ProgressBar::hidden()
    };
    let style = ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✔"])
        .template("{spinner:.cyan} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());
    pb.set_style(style);
    pb.set_message(msg);
    pb
}

/// Mark a spinner finished with a green checkmark and a final message.
pub fn spinner_finish(pb: &ProgressBar, msg: impl Into<String>) {
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_strings(&["✔"]),
    );
    pb.finish_with_message(msg.into());
}