# Relay

Secure cross-platform command router.

See [`product.md`](./product.md) for the product spec and [`roadmap.md`](./roadmap.md) for milestones.

## Getting Started

```bash
# 1. Initialise ~/.relay (and ~/.relay/bin where shims live).
relay init

# 2. Register your first command.
#    Prefix form (most common): `v dev` -> `vite dev`
relay add v vite

#    Exact form: `vd` -> `vite dev`
relay add vd vite dev

# 3. Put ~/.relay/bin on PATH.
#    Unix/macOS: add this to ~/.bashrc / ~/.zshrc / etc.
export PATH="$HOME/.relay/bin:$PATH"

#    Windows PowerShell (current session):
$env:PATH = "$env:USERPROFILE\.relay\bin;$env:PATH"
#    (persistent: edit your $PROFILE or use `setx PATH`)

# 4. Use it.
v dev          # actually runs `vite dev`
v build
vd             # runs `vite dev` (exact)

# 5. Inspect.
relay list
relay info v
relay doctor   # validate PATH / shims / config
```

If `relay doctor` reports missing or orphan shims, run `relay doctor --fix`
(or `relay rebuild-shims` for a full reset from config).

## Build

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Layout

```
src/
├── main.rs           # entry point
├── lib.rs            # library root (re-exports)
├── cli.rs            # clap definitions and dispatch
├── error.rs          # RelayError + Result alias
├── config/           # on-disk config (paths, load/save, schema)
├── registry/         # command registry (add/remove/update/list/info/init)
├── runner/           # `relay run <name> [args...]` — the security-critical path
├── shim/             # shim generation and sync under ~/.relay/bin
└── doctor/           # `relay doctor` — environment validation

tests/                # registry / shim / cli integration tests
```

## Status

v0.1 MVP. CLI surface complete; shims generate and sync on every mutation.
`export` / `import` are scheduled for v0.4.
