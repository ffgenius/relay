# Relay

Secure cross-platform command router.

See [`product.md`](./product.md) for the product spec and [`roadmap.md`](./roadmap.md) for milestones.

## Build

```bash
cargo build
cargo test
```

## Layout

```
src/
├── main.rs           # entry point
├── lib.rs            # library root (re-exports)
├── cli.rs            # clap definitions and dispatch
├── error.rs          # RelayError + Result alias
├── config/           # on-disk config (paths, load/save, schema)
│   ├── mod.rs
│   ├── paths.rs
│   └── schema.rs
├── registry/         # command registry (add/remove/update/list/info)
│   └── mod.rs
├── runner/           # `relay run <name> [args...]` — the security-critical path
│   └── mod.rs
├── shim/             # shim generation under ~/.relay/bin
│   └── mod.rs
└── doctor/           # `relay doctor` — environment validation
    └── mod.rs

tests/                # integration tests via assert_cmd
```

## Status

v0.1 MVP scaffolding. CLI surface is wired through clap; most subcommands return
`RelayError::Unimplemented` until their module is filled in.
