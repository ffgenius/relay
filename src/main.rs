use std::process::ExitCode;

fn main() -> ExitCode {
    // On Windows, cmd.exe / older Windows Terminal builds don't process
    // ANSI escape codes by default — they render them as literal `␛[1m`
    // glyphs. Calling SetConsoleMode(ENABLE_VIRTUAL_TERMINAL_PROCESSING)
    // once at startup flips the terminal into VT mode for the rest of
    // the process. Best-effort: if it fails (legacy Windows < 10, or
    // stdout isn't a real console), we silently fall back and colors
    // will be stripped by `if_supports_color` in ui.rs.
    #[cfg(windows)]
    {
        let _ = enable_ansi_support::enable_ansi_support();
    }

    match relay::cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            relay::ui::err(format!("{err}"));
            // Print the full cause chain for debugging — keep it dimmed
            // since the top-level message is the actionable one.
            let mut source = std::error::Error::source(&err);
            while let Some(cause) = source {
                eprintln!("  caused by: {cause}");
                source = cause.source();
            }
            ExitCode::FAILURE
        }
    }
}
