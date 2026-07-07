# Relay

> **Secure cross-platform command router.** Type `v dev`, run `vite dev`. Without shell aliases. Without surprises.

[![npm](https://img.shields.io/npm/v/@ffgenius/relay.svg)](https://www.npmjs.com/package/@ffgenius/relay)
[![homebrew](https://img.shields.io/badge/homebrew-ffgenius%2Ftap-orange.svg)](https://github.com/ffgenius/homebrew-tap)
[![license](https://img.shields.io/npm/l/@ffgenius/relay.svg)](./LICENSE)

[中文文档](./README.zh.md)

---

## Why Relay?

Shell aliases work, until they don't:

- `alias` rules differ between bash / zsh / fish / PowerShell. Sharing config across shells is painful.
- Aliases run **inside a shell** — `alias g='git'` is just a string macro. There's no guarantee that what you type actually maps to the binary you think it does.
- They live in dotfiles. Move to a new machine, lose them. Switch shells, lose them.

**Relay routes commands without invoking a shell.** Each alias becomes a tiny launcher (a "shim") that calls relay's runner, which spawns the target program directly — no `sh -c`, no string evaluation, no surprises.

```bash
relay add v vite      # register
v dev                 # runs `vite dev` — never goes through a shell
v build               # runs `vite build`
```

Same syntax on Linux, macOS, and Windows. Same config file. Same behaviour.

---

## Install

Pick the method that fits your workflow — **no Node.js required** for Homebrew or the shell installer.

### Homebrew (macOS / Linux)

```bash
brew tap ffgenius/tap
brew install relay
relay init
```

[Homebrew](https://brew.sh) installs the binary and keeps it up to date with `brew upgrade`.

### Shell installer (curl / wget)

```bash
curl -fsSL https://raw.githubusercontent.com/ffgenius/relay/master/install.sh | sh
```

Or with `wget`:

```bash
wget -qO- https://raw.githubusercontent.com/ffgenius/relay/master/install.sh | sh
```

The script detects your OS and architecture, downloads the right binary from
GitHub Releases, installs it to `~/.relay/bin/`, and runs `relay init` for
automatic shell integration (bash / zsh / fish).

**Options:** `--version 0.1.0` to pin a version; `--no-init` to skip
shell-profile changes.

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/ffgenius/relay/master/install.ps1 | iex
```

Same behaviour as the shell installer: downloads the right binary, installs to
`~\.relay\bin\`, and adds it to your user `PATH` via the registry.

**Options:** `-Version 0.1.0` to pin a version; `-NoInit` to skip PATH setup.

### npm

```bash
npm install -g @ffgenius/relay
```

The npm package selects the right binary for your platform automatically
(`linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`, `win32-x64`,
`win32-arm64`).

### After install

All methods above run `relay init` for you (unless you opt out). This creates
`~/.relay/` and adds `~/.relay/bin` to your `PATH`.

**Open a new terminal** for the PATH change to take effect — then you're ready
to go.

---

## Quick start

```bash
# Prefix alias — `v <anything>` runs `vite <anything>`.
relay add v vite

# Prefix alias with default args — `gt <anything>` runs `git clone <anything>`.
relay add gt git -p clone

# Exact alias — `vd` always runs `vite dev`, no arguments accepted.
relay add vd vite dev

# Use them.
v dev                 # → vite dev
v build               # → vite build
gt https://example.com/repo.git  # → git clone https://example.com/repo.git
vd                    # → vite dev

# Inspect.
relay list            # all aliases (also: relay ls)
relay info v          # details for one alias
relay discover vite   # aliases grouped by target program

# Store and run shell snippets (with cross-shell auto-translation).
relay snippet add goback "cd ../"
relay snippet run goback --dry-run

# Snippets support {{0}} {{1}} … placeholders — pass args at runtime.
relay snippet add killport --shell powershell "Get-NetTCPConnection -LocalPort {{0}} ^| ForEach-Object { Stop-Process -Id `$_.OwningProcess -Force }"
relay snippet run killport --dry-run 4600
# → Get-NetTCPConnection -LocalPort 4600 | ...

# Diagnose.
relay doctor          # check PATH, shims, config
relay doctor --fix    # auto-repair missing shims and PATH entries
```

---

## Concepts

Relay has three kinds of items:

### Prefix alias

`relay add <name> <program>` — every argument you type after `<name>` is forwarded.

```bash
relay add v vite
v dev      # → vite dev
v build    # → vite build
v --help   # → vite --help
```

You can also register a prefix alias **with default arguments** using `--prefix` / `-p`.
The default args are always included, and any extra runtime args are appended after them:

```bash
relay add gt git -p clone
gt https://example.com/repo.git          # → git clone https://example.com/repo.git
gt --depth 1 https://example.com/repo.git # → git clone --depth 1 https://example.com/repo.git
```

### Exact alias

`relay add <name> <program> <args...>` (without `--prefix`) — the arguments are baked in; runtime args are rejected.

```bash
relay add vd vite dev
vd         # → vite dev (always)
vd preview # → error: exact command does not accept extra arguments
```

Use **prefix** for tools you call with many subcommands (`v`, `g`, `n`). Use **prefix with args** when the command itself has a fixed subcommand but you want to pass extra flags (`gt` for `git clone`). Use **exact** for one-liners you run all the time with no variation (`vd`, `gp`, `nci`).

### Snippet

`relay snippet add <name> <content...>` — store an arbitrary shell code fragment. Unlike regular aliases (which bypass the shell), snippets are executed through a shell interpreter and support **automatic cross-shell translation** via [polysh](https://github.com/ffgenius/polysh).

```bash
# Create a snippet — relay auto-detects your current shell.
relay snippet add goback "cd ../"

# Run it — if your current shell differs from the one it was written in,
# relay translates the command automatically (Unix ↔ PowerShell ↔ CMD).
relay snippet run goback

# Preview the translated command without executing.
relay snippet run goback --dry-run
```

**Placeholders:** Use `{{0}}` `{{1}}` … in snippet content and pass arguments at runtime:

```bash
relay snippet add greet --shell powershell "Write-Host Hello {{0}}"
relay snippet run greet --dry-run World        # → Write-Host Hello World
relay snippet add killport --shell powershell "Get-NetTCPConnection -LocalPort {{0}} ^| ForEach-Object { Stop-Process -Id `$_.OwningProcess -Force }"
killport 4600                                   # shim forwarding works too
```

If a placeholder index exceeds the number of arguments provided, relay returns a clear error.

**Why snippets?** Commands like `cd`, `export`, complex pipes, and shell built-ins can't work through relay's direct-execution model. Snippets fill that gap while keeping cross-shell portability.

---

## Command reference

### Basics

| Command | Description |
|---|---|
| `relay init` | Create `~/.relay`, write empty config, add `~/.relay/bin` to PATH |
| `relay add <name> <program> [args...]` | Register an alias (prefix if no args or `--prefix`, exact otherwise) |
| `relay add <name> <program> -p [args...]` | Register a prefix alias with default args (runtime args appended) |
| `relay remove <name>` (alias: `rm`) | Delete an alias |
| `relay update <name> <program> [args...]` | Replace an existing alias (accepts `--prefix` / `-p`) |
| `relay list` (alias: `ls`) | List all aliases by name |
| `relay info <name>` | Show details for one alias |
| `relay clear` (alias: `cls`) | Remove every alias (asks for confirmation) |
| `relay clear --yes` | Same, no confirmation |

### Discovery

| Command | Description |
|---|---|
| `relay discover` | Group aliases by their target program |
| `relay discover <program>` | Show all aliases targeting `<program>` |

### Backup & sync

| Command | Description |
|---|---|
| `relay export` | Print config to stdout (YAML). Includes snippets by default |
| `relay export -o <file>` | Write to file (`.yaml` auto-appended if missing) |
| `relay export --no-snippet` | Export only commands, exclude snippets |
| `relay import <file>` | Merge another config. Snippets are **skipped by default** for security |
| `relay import <file> --overwrite` | Merge, overwriting conflicting aliases |
| `relay import <file> --allow-snippet` | Also import snippets from the file |
| `relay sync init` | Create a private GitHub Gist and link this machine to it |
| `relay sync link <gist_id>` | Link this machine to an existing Gist |
| `relay sync unlink` | Forget the linked Gist on this machine (remote Gist is kept) |
| `relay sync push` | Upload local config (commands + snippets) to the linked Gist |
| `relay sync push --no-snippet` | Upload only commands, exclude snippets |
| `relay sync pull` | Download config from the Gist. Snippets **skipped by default** |
| `relay sync pull --allow-snippet` | Download and include snippets |
| `relay sync status` | Show sync status, command and snippet counts |

### Snippets

| Command | Description |
|---|---|
| `relay snippet add <name> <content...>` | Create a snippet (auto-detects current shell) |
| `relay snippet add <name> <content...> --shell <d>` | Create with explicit shell dialect (`unix`, `powershell`, `cmd`) |
| `relay snippet add <name> <content...> --desc <d>` | Create with a description |
| `relay snippet remove <name>` (alias: `rm`) | Delete a snippet |
| `relay snippet list` (alias: `ls`) | List all snippets |
| `relay snippet info <name>` | Show full details of one snippet |
| `relay snippet edit <name> --content <c>` | Update a snippet's content |
| `relay snippet edit <name> --desc <d>` | Update description (pass `""` to clear) |
| `relay snippet edit <name> --shell <d>` | Change the shell dialect |
| `relay snippet run <name> [args...]` | Execute a snippet (auto-translates to current shell). `args` replace `{{0}}` `{{1}}` … placeholders |
| `relay snippet run <name> --dry-run [args...]` | Print the substituted & translated command without executing |
| `relay snippet run <name> --no-translate [args...]` | Run as-is, skip cross-shell translation (placeholders still apply) |
| `relay snippet clear` | Remove all snippets (asks for confirmation) |
| `relay snippet clear --yes` | Same, no confirmation |

### System

| Command | Description |
|---|---|
| `relay doctor` | Validate PATH, shims, config |
| `relay doctor --fix` | Re-generate missing shims and auto-add PATH entries |
| `relay rebuild` | Full reset: regenerate every shim from the current config |

---

## Sync across machines

Relay syncs to a **private GitHub Gist** through your existing `gh` CLI session. No new tokens to manage.

**On your first machine:**

```bash
gh auth login                 # if you haven't already
relay add v vite              # register a few aliases
relay add g git
relay sync init               # → creates a Gist, prints its ID
```

**On your second machine:**

```bash
gh auth login
relay sync link <gist_id>     # the ID from `sync init` above
relay sync pull               # downloads the aliases, regenerates shims
```

**Day-to-day:**

```bash
relay add p pnpm              # add a new alias on machine A
relay sync push               # upload the change
# ...later, on machine B:
relay sync pull               # pull the change
```

`relay sync status` shows whether your local config is in sync with the Gist; `pull` warns before overwriting un-pushed local changes.

---

## Security model

Relay's whole point is to be safe by construction — running `v dev` should be **boringly equivalent** to running `vite dev` directly. The four principles below are enforced at code level:

> **Principle 1 — Relay does not execute a shell (except for snippets).**
> Regular aliases use `std::process::Command` to spawn the target binary directly — no `sh -c`, no `cmd /c`, no `powershell -Command`. **Snippets are the deliberate exception**: since they are shell code by nature, they run through a shell interpreter. This is why import/pull require `--allow-snippet` — snippets are opt-in trusted code.

> **Principle 2 — Relay does not execute strings.**
> An alias is a `(program, args)` tuple. There is no `exec: "vite dev && rm -rf /"` field. Strings as commands are not a representable state.

> **Principle 3 — Relay only executes registered binaries that exist.**
> `relay add` calls `which(<program>)` and refuses to register anything that isn't on `PATH`. The path separator (`/`, `\`) is also rejected — only bare command names allowed — so a malicious gist can't sneak `/tmp/evil-cargo` into your config via `relay sync pull`.

> **Principle 4 — Shells are on the blocklist.**
> `sh`, `bash`, `zsh`, `cmd`, `powershell`, `pwsh` cannot be registered as the target of an alias. Even if you try `relay add x sh`, it's refused.

These rules also mean Relay can never *be* the attack — there's no shell escape in the path between your shim and your binary.

---

## Configuration

Everything lives in `~/.relay/`:

```
~/.relay/
├── config.yaml          # registered aliases + snippets
├── sync-state.yaml      # (optional) linked Gist ID + sync hash
└── bin/                 # generated shims; this dir goes on PATH
    ├── v                # or v.cmd on Windows
    ├── vd
    └── ...
```

`config.yaml` is intentionally readable and hand-editable (re-run `relay rebuild` after manual edits):

```yaml
version: 1
commands:
  v:
    type: prefix
    program: vite
  vd:
    type: exact
    program: vite
    args:
      - dev
snippets:
  goback:
    type: snippet
    content: "cd ../"
    shell: unix
  serve:
    type: snippet
    content: "python3 -m http.server 8080"
    shell: unix
    description: "start a local file server"
```

---

## Troubleshooting

### `n: command not found` even though `relay add n nvm` succeeded

Your shell hasn't picked up the new PATH yet. Run:

```bash
relay doctor
```

If it says `shim dir is NOT on PATH`, run `relay doctor --fix` and then **open a new terminal**.

On Windows, the registry write may not propagate to existing `cmd` windows until you log out and back in. Open a fresh terminal first; if it's still wrong, run `relay doctor` from the fresh terminal.

### First execution returns `EPERM` (Windows)

Windows Defender / SmartScreen is scanning the newly-installed `relay.exe` on its first run. The scan releases the file within a second or two — just re-run the same command. This only happens once per install.

### `PATH` is full / changes don't appear

Windows truncates the combined `PATH` at process creation if it exceeds ~2047 characters. `relay init` writes the shim directory to the **front** of the user `PATH` to dodge truncation, but a heavily-loaded `PATH` can still hide entries. `relay doctor` warns when the user `PATH` exceeds 1900 characters.

### Sync says `gh: not authenticated`

Run `gh auth login` once. Relay piggybacks on your GitHub CLI session — it never asks for tokens directly.

---

## Contributing

```bash
git clone https://github.com/ffgenius/relay
cd relay
cargo build
cargo test
```

Issues and PRs welcome.

## License

[MIT](./LICENSE) © Bin
