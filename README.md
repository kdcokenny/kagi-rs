# Kagi Rust Workspace

Rust-first monorepo for Kagi tooling.

## Scope in this pass

- ✅ `sdk/` crate (implemented)
- ⏳ `cli/` crate (planned, not implemented)
- ⏳ `mcp/` crate (planned, not implemented)

The workspace is intentionally SDK-first. CLI and MCP crates will be added later on top of this SDK.

## Protocol surfaces made explicit

The SDK intentionally exposes two separate surfaces:

1. **Official Kagi API**
   - Auth: `Authorization: Bot <token>`
   - Routes: `/api/v0/*`, `/api/v1/*` (in-scope subset only)
   - Response style: JSON envelopes

2. **Session-token-backed web surface**
   - Auth: `Cookie: kagi_session=<token>`
   - Routes: `/html/search`, `/mother/summary_labs`, `/mother/summary_labs/`
   - Response style: HTML + stream parsing

See `docs/endpoint-auth-version-matrix.md` for the v1 source-of-truth route/auth/version matrix.

## Verification commands

Run from repository root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
