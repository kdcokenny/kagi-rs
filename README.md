# kagi-rs

Rust-native workspace for building Kagi integrations with a typed SDK and an MCP server for agents.

## Workspace status

| Path | Package | Status | Purpose |
|---|---|---|---|
| `sdk/` | `kagi-sdk` | ✅ Available | Rust SDK with explicit official-api and session-web surfaces |
| `mcp/` | `kagi-mcp` | ✅ Available | MCP server (v1: stdio-only, tools-only) built on `kagi-sdk` |
| `cli/` | *(planned)* | ⏳ Planned | Reserved path for a future end-user CLI crate |
| `docs/` | n/a | ✅ Available | Reference and maintainer/operator documentation |

## Quickstart (canonical)

Run the MCP server locally from source:

```bash
KAGI_API_KEY=... cargo run -p kagi-mcp
```

This starts a stdio MCP server you can register in Claude Code, Codex, or OpenCode.

## Choose your path

- **Building Rust apps/services:** start in [`sdk/README.md`](./sdk/README.md)
- **Connecting agent hosts to Kagi:** start in [`mcp/README.md`](./mcp/README.md)
- **CLI roadmap/status:** see [`cli/README.md`](./cli/README.md)
- **Docs index (user + reference + maintainer):** see [`docs/README.md`](./docs/README.md)

## Two surfaces, one SDK

`kagi-sdk` keeps Kagi's protocol surfaces explicit:

| Surface | Auth | Route families | Response shape |
|---|---|---|---|
| Official API | `Authorization: Bot <token>` | `/api/v0/*`, `/api/v1/*` (in-scope subset) | JSON envelopes |
| Session web | `Cookie: kagi_session=<token>` | `/html/search`, `/mother/summary_labs` | HTML / JSON / framed stream |

Authoritative endpoint/auth/version coverage: [`docs/endpoint-auth-version-matrix.md`](./docs/endpoint-auth-version-matrix.md).

## Docs map

| Need | Go to |
|---|---|
| Workspace onboarding | [`README.md`](./README.md) |
| SDK usage and API surfaces | [`sdk/README.md`](./sdk/README.md) |
| MCP host setup and agent usage | [`mcp/README.md`](./mcp/README.md) |
| CLI planned status | [`cli/README.md`](./cli/README.md) |
| Endpoint/auth/version source of truth | [`docs/endpoint-auth-version-matrix.md`](./docs/endpoint-auth-version-matrix.md) |
| Release + workflow policy (maintainer) | [`docs/releasing.md`](./docs/releasing.md) |

## Development commands

Run from repository root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Contributor and maintainer docs

Contributor onboarding remains in package READMEs; maintainer/operator policy lives in [`docs/releasing.md`](./docs/releasing.md).
