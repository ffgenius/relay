use std::process::ExitCode;

fn main() -> ExitCode {
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
