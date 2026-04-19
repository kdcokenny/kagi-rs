# kagi-cli (planned)

This directory is reserved for a future end-user CLI crate built on top of `kagi-sdk`.

## Current status

| Item | Status |
|---|---|
| `cli/` path reserved | ✅ |
| Workspace membership (`Cargo.toml`) | ⏳ Not yet included |
| Published crate | ⏳ Not available |

## Planned direction

`kagi-cli` is intended to provide a command-line UX for common Kagi workflows without requiring custom Rust code.

Expected foundation:

- `kagi-sdk` for protocol/auth correctness
- Shared endpoint scope from [`../docs/endpoint-auth-version-matrix.md`](../docs/endpoint-auth-version-matrix.md)

## What to use today

- Rust/library usage: [`../sdk/README.md`](../sdk/README.md)
- Agent-host integration: [`../mcp/README.md`](../mcp/README.md)
- Workspace overview: [`../README.md`](../README.md)

## Maintainer notes

Release/workflow policy for active crates is documented in [`../docs/releasing.md`](../docs/releasing.md).
