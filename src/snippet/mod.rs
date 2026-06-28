//! Snippet registry — CRUD for shell code fragments stored in
//! `~/.relay/config.yaml`.
//!
//! Handler functions take `&Paths` so tests can inject a `TempDir`.
//! Shim regeneration is handled centrally from `cli::dispatch` after
//! successful mutations.

use std::process::{Command, Stdio};

use crate::{
    config::{self, schema::ShellDialect, schema::Snippet, schema::SnippetKind, Paths},
    ui, RelayError, Result,
};

// ─── CRUD ────────────────────────────────────────────────────────────────

/// `relay snippet add <name> <content...>` — create a new snippet.
///
/// Auto-detects the current shell via polysh unless `shell` is explicitly
/// provided. The content is built by joining trailing args with spaces.
pub fn add(
    paths: &Paths,
    name: &str,
    content_words: &[String],
    shell: Option<&str>,
    desc: Option<&str>,
) -> Result<()> {
    validate_name(name)?;

    let mut config = config::load(paths)?;

    // Check for name conflict with commands.
    if config.commands.contains_key(name) {
        return Err(RelayError::SnippetNameConflict(name.to_string()));
    }
    if config.snippets.contains_key(name) {
        return Err(RelayError::SnippetExists(name.to_string()));
    }

    let content = content_words.join(" ");
    let dialect = match shell {
        Some(s) => parse_dialect(s)?,
        None => detect_current_dialect(),
    };

    config.snippets.insert(
        name.to_string(),
        Snippet {
            kind: SnippetKind::Snippet,
            content: content.clone(),
            shell: dialect,
            description: desc.map(String::from),
        },
    );

    config::save(paths, &config)?;
    let shell_str = dialect_name(dialect);
    let preview = if content.len() > 60 {
        format!("{}...", &content[..57])
    } else {
        content
    };
    ui::ok(format!("added snippet {name} [{shell_str}] → {preview}"));
    Ok(())
}

/// `relay snippet remove <name>` — delete a snippet.
pub fn remove(paths: &Paths, name: &str, auto_yes: bool) -> Result<()> {
    let mut config = config::load(paths)?;
    if config.snippets.remove(name).is_none() {
        return Err(RelayError::UnknownSnippet(name.to_string()));
    }

    if !auto_yes && !prompt_confirm(&format!("Remove snippet '{name}'?")) {
        ui::line("cancelled.");
        return Ok(());
    }

    config::save(paths, &config)?;
    ui::ok(format!("removed snippet {name}"));
    Ok(())
}

/// `relay snippet list` — print all registered snippets.
pub fn list(paths: &Paths) -> Result<()> {
    let config = config::load(paths)?;
    if config.snippets.is_empty() {
        ui::line("(no snippets registered — try `relay snippet add name content`)");
        return Ok(());
    }
    for (name, snip) in &config.snippets {
        snippet_line(name, snip);
    }
    Ok(())
}

/// `relay snippet info <name>` — show full details of one snippet.
pub fn info(paths: &Paths, name: &str) -> Result<()> {
    let config = config::load(paths)?;
    let snip = config
        .snippets
        .get(name)
        .ok_or_else(|| RelayError::UnknownSnippet(name.to_string()))?;

    ui::field("name", name);
    ui::field("type", "snippet");
    ui::field("shell", dialect_name(snip.shell));
    ui::field("content", &snip.content);
    if let Some(ref desc) = snip.description {
        ui::field("desc", desc);
    }

    // Warn about the subprocess limitation for state-changing commands.
    if has_stateful_commands(&snip.content) {
        ui::blank();
        ui::note(
            "this snippet contains commands that modify shell state (cd, export, set, etc.).\n       \
             They will NOT persist in your current session because the snippet runs in a subprocess.\n       \
             Use `relay snippet run --dry-run` to see the translated command and run it manually.",
        );
    }

    Ok(())
}

/// `relay snippet edit <name>` — update a snippet's content, description, or shell.
pub fn edit(
    paths: &Paths,
    name: &str,
    content: Option<&str>,
    desc: Option<&str>,
    shell: Option<&str>,
) -> Result<()> {
    let mut config = config::load(paths)?;
    let snip = config
        .snippets
        .get_mut(name)
        .ok_or_else(|| RelayError::UnknownSnippet(name.to_string()))?;

    let mut changes = Vec::new();

    if let Some(c) = content {
        snip.content = c.to_string();
        changes.push("content");
    }
    match desc {
        Some("") => {
            snip.description = None;
            changes.push("desc (cleared)");
        }
        Some(d) => {
            snip.description = Some(d.to_string());
            changes.push("desc");
        }
        None => {}
    }
    if let Some(s) = shell {
        snip.shell = parse_dialect(s)?;
        changes.push("shell");
    }

    if changes.is_empty() {
        ui::line("(no changes)");
        return Ok(());
    }

    config::save(paths, &config)?;
    ui::ok(format!("updated snippet {name}: {}", changes.join(", ")));
    Ok(())
}

/// `relay snippet clear` — remove all snippets.
pub fn clear(paths: &Paths, auto_yes: bool) -> Result<()> {
    let mut config = config::load(paths)?;
    let count = config.snippets.len();
    if count == 0 {
        ui::line("(no snippets registered)");
        return Ok(());
    }

    if !auto_yes
        && !prompt_confirm(&format!(
            "This will remove all {count} snippet(s). Continue?"
        ))
    {
        ui::line("cancelled.");
        return Ok(());
    }

    config.snippets.clear();
    config::save(paths, &config)?;
    ui::ok(format!("cleared {count} snippet(s)"));
    Ok(())
}

// ─── Run (cross-shell execution) ─────────────────────────────────────────

/// `relay snippet run <name>` — execute a snippet, optionally translating
/// it to the current shell dialect via polysh.
///
/// Pass `target` to override the auto-detected current dialect — useful
/// for testing cross-shell translation results (e.g. `--target unix`).
/// Pass `force` to run a best-effort translation even when polysh cannot
/// fully translate all segments.
pub fn run(paths: &Paths, name: &str, dry_run: bool, no_translate: bool, target: Option<&str>, force: bool) -> Result<()> {
    let config = config::load(paths)?;
    let snip = config
        .snippets
        .get(name)
        .ok_or_else(|| RelayError::UnknownSnippet(name.to_string()))?;

    let current_dialect = match target {
        Some(t) => parse_dialect(t)?,
        None => detect_current_dialect(),
    };
    let stored_dialect = snip.shell;

    let command = if no_translate || current_dialect == stored_dialect {
        snip.content.clone()
    } else {
        translate_content(&snip.content, stored_dialect, current_dialect, force)?
    };

    if dry_run {
        ui::header("shell  :", dialect_name(current_dialect));
        ui::header("stored :", dialect_name(stored_dialect));
        ui::header("command:", &command);
        return Ok(());
    }

    execute_in_shell(&command, current_dialect)
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(RelayError::InvalidCommandName(
            name.to_string(),
            "name is empty",
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(RelayError::InvalidCommandName(
            name.to_string(),
            "only ASCII letters, digits, '-' and '_' are allowed",
        ));
    }
    Ok(())
}

/// Detect the current shell dialect via polysh.
pub fn detect_current_dialect() -> ShellDialect {
    let info = polysh::detector::detect_shell();
    info.dialect.into()
}

/// Parse a user-supplied dialect name.
pub fn parse_dialect(s: &str) -> Result<ShellDialect> {
    match s.to_lowercase().as_str() {
        "unix" | "bash" | "sh" | "zsh" | "fish" => Ok(ShellDialect::Unix),
        "powershell" | "ps" | "pwsh" => Ok(ShellDialect::PowerShell),
        "cmd" | "batch" | "dos" => Ok(ShellDialect::Cmd),
        _ => Err(RelayError::InvalidShell(s.to_string())),
    }
}

/// Human-readable dialect name.
pub fn dialect_name(d: ShellDialect) -> &'static str {
    match d {
        ShellDialect::Unix => "unix",
        ShellDialect::PowerShell => "powershell",
        ShellDialect::Cmd => "cmd",
    }
}

/// Translate snippet content from `source` dialect to `target` dialect
/// using polysh.
///
/// Returns an error when the translation has unsupported segments, to
/// prevent silently executing broken/garbled commands. Use `--force` to
/// override this safety check.
fn translate_content(content: &str, source: ShellDialect, target: ShellDialect, force: bool) -> Result<String> {
    let src: polysh::mappings::Dialect = source.into();
    let tgt: polysh::mappings::Dialect = target.into();

    // Build ShellInfo matching the target environment so polysh can handle
    // dialect-specific escaping (PowerShell backticks, connector rewriting).
    let shell = polysh::detector::ShellInfo {
        dialect: tgt,
        supports_conditional_connectors: true,
        needs_unix_translation: tgt != polysh::mappings::Dialect::Unix,
        target: tgt,
        version: None,
    };

    let registry = polysh::mappings::MappingRegistry::new();
    let translated =
        polysh::translator::translate_with_registry(content, src, tgt, &shell, &registry);

    if translated != content {
        let lint = polysh::translator::lint_command(content);
        if !lint.unsupported.is_empty() {
            let names: Vec<String> = lint
                .unsupported
                .iter()
                .map(|s| {
                    // Extract just the unknown command name for readability.
                    if let Some(pos) = s.find("(unknown command: '") {
                        let cmd = &s[pos + "(unknown command: '".len()..];
                        let cmd = cmd.trim_end_matches("')");
                        format!("'{}'", cmd)
                    } else {
                        s.clone()
                    }
                })
                .collect();

            if force {
                ui::warn(format!(
                    "incomplete translation ({}). Running best-effort result.",
                    names.join(", ")
                ));
            } else {
                return Err(RelayError::Other(anyhow::anyhow!(
                    "Cannot translate {}. Use --no-translate to run in the stored dialect, or --force to run the best-effort translation.\n       Unsupported: {}",
                    names.join(", "),
                    lint.unsupported.join("\n       → ")
                )));
            }
        }
    }

    Ok(translated)
}

/// Execute a command string through the appropriate shell interpreter.
fn execute_in_shell(command: &str, dialect: ShellDialect) -> Result<()> {
    let status = match dialect {
        ShellDialect::Unix => {
            let mut child = Command::new("sh")
                .args(["-c", command])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| RelayError::Other(anyhow::anyhow!("failed to spawn sh: {e}")))?;
            child
                .wait()
                .map_err(|e| RelayError::Other(anyhow::anyhow!("sh exited with error: {e}")))?
        }
        ShellDialect::PowerShell => {
            let mut child = Command::new("powershell")
                .args(["-Command", command])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| {
                    RelayError::Other(anyhow::anyhow!("failed to spawn powershell: {e}"))
                })?;
            child.wait().map_err(|e| {
                RelayError::Other(anyhow::anyhow!("powershell exited with error: {e}"))
            })?
        }
        ShellDialect::Cmd => {
            let mut child = Command::new("cmd")
                .args(["/c", command])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| RelayError::Other(anyhow::anyhow!("failed to spawn cmd: {e}")))?;
            child
                .wait()
                .map_err(|e| RelayError::Other(anyhow::anyhow!("cmd exited with error: {e}")))?
        }
    };

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(RelayError::Other(anyhow::anyhow!(
            "snippet exited with code {code}"
        )));
    }
    Ok(())
}

/// Check for commands that modify parent shell state (cd, export, set, etc.).
fn has_stateful_commands(content: &str) -> bool {
    let triggers = [
        "cd ", "cd\t", "export ", "set ", "unset ", "alias ", "source ", ". ", "exec ", "ulimit ",
        "umask ",
    ];
    let lower = content.to_lowercase();
    triggers
        .iter()
        .any(|t| lower.starts_with(t) || lower.contains(t))
}

/// Render one snippet row the way `relay snippet list` wants it.
fn snippet_line(name: &str, snip: &Snippet) {
    use owo_colors::{OwoColorize, Stream};

    let preview = if snip.content.len() > 48 {
        format!("{}...", &snip.content[..45])
    } else {
        snip.content.clone()
    };
    let shell = dialect_name(snip.shell);

    println!(
        "  {name:<16} {kind} {shell:<11} {preview}",
        name = name.if_supports_color(Stream::Stdout, |t| t.bold()),
        kind = "[snippet]".if_supports_color(Stream::Stdout, |t| t.dimmed()),
        shell = shell.if_supports_color(Stream::Stdout, |t| t.dimmed()),
        preview = preview,
    );
}

/// Prompt the user for a yes/no answer via stdin.
fn prompt_confirm(prompt: &str) -> bool {
    use std::io::{BufRead, Write};
    print!("{prompt} [Y/N] ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).ok();
    let trimmed = line.trim().to_lowercase();
    trimmed == "y" || trimmed == "yes"
}
