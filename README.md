# kagi-rs

Rust-native tooling workspace for Kagi.

`kagi-rs` currently ships an SDK crate and an MCP crate built on top of that SDK.

## Workspace status

| Path | Crate | Status | Notes |
|---|---|---|---|
| `sdk/` | `kagi-sdk` | ✅ Available | Typed Rust SDK with explicit official-api and session-web surfaces |
| `cli/` | *(planned)* | ⏳ Planned | Reserved for a future end-user command-line interface |
| `mcp/` | `kagi-mcp` | ✅ Available | MCP stdio server exposing `kagi_search` and `kagi_summarize` |

## Quickstart (workspace)

```bash
# from repo root
cargo test --workspace

# run SDK examples
cargo run -p kagi-sdk --example bot_token
cargo run -p kagi-sdk --example session_token

# run MCP stdio server
KAGI_API_KEY=... cargo run -p kagi-mcp
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
- **Lean foundation**: keep dependencies and surface area focused while expanding deliberately.

## Current workspace layout

```text
kagi-rs/
├── sdk/      # implemented Rust SDK crate
├── mcp/      # implemented Rust MCP stdio server crate
└── docs/     # endpoint/auth/version matrix and project docs
```

## Planned additions

- `cli/`: future command-line crate built on `kagi-sdk`

`cli/` remains a planned next step and is not an active workspace crate yet.

## Development commands

Run from repository root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
