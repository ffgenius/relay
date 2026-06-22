# Relay

Secure cross-platform command router.

See [`product.md`](./product.md) for the product spec and [`roadmap.md`](./roadmap.md) for milestones.

## Install

```bash
npm install -g @ffgenius/relay
# or: pnpm add -g @ffgenius/relay
# or: yarn global add @ffgenius/relay
```

Supported platforms: `linux-x64`, `linux-arm64`, `darwin-x64`, `darwin-arm64`,
`win32-x64`, `win32-arm64`. npm installs the matching binary automatically via
optional dependencies — only one platform's binary ends up on disk.

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
src/                  # Rust crate
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

npm/                  # distribution
├── relay/            # @ffgenius/relay — node wrapper + optionalDependencies
└── platforms/        # @ffgenius/relay-<platform>-<arch> × 6 — binaries

.github/workflows/
└── release.yml       # triggered on `v*.*.*` tags; builds + publishes
```

## Release

```bash
# Bump version in Cargo.toml and all npm/**/package.json, commit, then:
git tag v0.0.1
git push origin v0.0.1
# → GitHub Actions builds 6 platform binaries and publishes all 7 npm packages.
```

Requires `NPM_TOKEN` (automation token with publish access on the `@ffgenius`
scope) set as a GitHub Actions secret.

## Status

v0.1 MVP. CLI surface complete; shims generate and sync on every mutation.
`export` / `import` are scheduled for v0.4.
