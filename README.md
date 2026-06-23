# Relay

> **Secure cross-platform command router.** Type `v dev`, run `vite dev`. Without shell aliases. Without surprises.

[![npm](https://img.shields.io/npm/v/@ffgenius/relay.svg)](https://www.npmjs.com/package/@ffgenius/relay)
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

```bash
npm install -g @ffgenius/relay
```

The npm package selects the right binary for your platform automatically (`linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`, `win32-x64`, `win32-arm64`).

After install, run:

```bash
relay init
```

This creates `~/.relay/` and adds `~/.relay/bin` to your `PATH`. **Open a new terminal** for the PATH change to take effect.

---

## Quick start

```bash
# Register a prefix alias — `v <anything>` runs `vite <anything>`.
relay add v vite

# Register an exact alias — `vd` always runs `vite dev`, no arguments.
relay add vd vite dev

# Use them.
v dev                 # → vite dev
v build               # → vite build
vd                    # → vite dev

# Inspect.
relay list            # all aliases (also: relay ls)
relay info v          # details for one alias
relay discover vite   # aliases grouped by target program

# Diagnose.
relay doctor          # check PATH, shims, config
relay doctor --fix    # auto-repair missing shims and PATH entries
```

---

## Concepts

Relay has two kinds of aliases:

### Prefix alias

`relay add <name> <program>` — every argument you type after `<name>` is forwarded.

```bash
relay add v vite
v dev      # → vite dev
v build    # → vite build
v --help   # → vite --help
```

### Exact alias

`relay add <name> <program> <args...>` — the arguments are baked in; runtime args are ignored.

```bash
relay add vd vite dev
vd         # → vite dev (always)
```

Use **prefix** for tools you call with many subcommands (`v`, `g`, `n`). Use **exact** for one-liners you run all the time (`vd`, `gp`, `nci`).

---

## Command reference

### Basics

| Command | Description |
|---|---|
| `relay init` | Create `~/.relay`, write empty config, add `~/.relay/bin` to PATH |
| `relay add <name> <program> [args...]` | Register an alias (prefix if no args, exact otherwise) |
| `relay remove <name>` (alias: `rm`) | Delete an alias |
| `relay update <name> <program> [args...]` | Replace an existing alias |
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
| `relay export` | Print config to stdout (YAML) |
| `relay export -o <file>` | Write to file (`.yaml` auto-appended if missing) |
| `relay import <file>` | Merge another config (existing aliases preserved) |
| `relay import <file> --overwrite` | Merge another config (existing aliases overwritten) |
| `relay sync init` | Create a private GitHub Gist and link this machine to it |
| `relay sync link <gist_id>` | Link this machine to an existing Gist |
| `relay sync unlink` | Forget the linked Gist on this machine (remote Gist is kept) |
| `relay sync push` | Upload local config to the linked Gist |
| `relay sync pull` | Download config from the Gist (overwrites local) |
| `relay sync status` | Show whether sync is configured and clean/dirty |

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

> **Principle 1 — Relay does not execute a shell.**
> No `sh -c`, no `cmd /c`, no `powershell -Command`. The runner uses `std::process::Command` to spawn the target binary directly.

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
├── config.yaml          # registered aliases
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
git config core.hooksPath .githooks   # enable pre-commit auto-format
cargo build
cargo test
```

The pre-commit hook runs `cargo fmt --all` on staged `.rs` files, so CI's
fmt check never bites you. Skip with `git commit --no-verify` if needed.

Issues and PRs welcome.

## License

[MIT](./LICENSE) © Bin
