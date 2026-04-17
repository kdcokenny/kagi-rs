# kagi-rs

Rust-native tooling workspace for Kagi.

`kagi-rs` is currently SDK-first: the core `kagi-sdk` crate is implemented and tested, with room reserved for future CLI and MCP crates built on top of it.

## Workspace status

| Path | Crate | Status | Notes |
|---|---|---|---|
| `sdk/` | `kagi-sdk` | ✅ Available | Typed Rust SDK with explicit official-api and session-web surfaces |
| `cli/` | *(planned)* | ⏳ Planned | Reserved for a future end-user command-line interface |
| `mcp/` | *(planned)* | ⏳ Planned | Reserved for a future MCP server crate |

> `cli/` and `mcp/` are not implemented in this repository yet.

## Quickstart (workspace)

```bash
# from repo root
cargo test --workspace

# run SDK examples
cargo run -p kagi-sdk --example bot_token
cargo run -p kagi-sdk --example session_token
```

For SDK usage details, see [`sdk/README.md`](./sdk/README.md).

## Dual-surface SDK model

The SDK makes Kagi's two protocol surfaces explicit:

1. **Official API surface**
   - Auth: `Authorization: Bot <token>`
   - Route families: `/api/v0/*`, `/api/v1/*` (in-scope subset only)
   - Response shape: JSON envelopes

2. **Session web surface**
   - Auth: `Cookie: kagi_session=<token>`
   - Routes: `/html/search`, `/mother/summary_labs`, `/mother/summary_labs/`
   - Response shape:
     - search: HTML parsing
     - summarize: JSON parsing
     - summarize_stream: framed stream parsing (advanced)

Authoritative route/auth/version scope lives in [`docs/endpoint-auth-version-matrix.md`](./docs/endpoint-auth-version-matrix.md).

## Design principles

- **Explicit protocol boundaries**: official API and session web are separate SDK entrypoints.
- **Fail-fast behavior**: unsupported auth/surface combinations and invalid response shapes fail loudly.
- **Typed inputs at boundaries**: request constructors parse and reject invalid data early.
- **Lean foundation**: keep dependencies and surface area focused while CLI/MCP are still future work.

## Current workspace layout

```text
kagi-rs/
├── sdk/      # implemented Rust SDK crate
└── docs/     # endpoint/auth/version matrix and project docs
```

## Planned additions

- `cli/`: future command-line crate built on `kagi-sdk`
- `mcp/`: future MCP server crate built on `kagi-sdk`

These are planned next steps, not active workspace crates today.

## Development commands

Run from repository root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
