# OpenCode agent notes for `kagi-rs`

## Workspace + invariants
- Workspace members are only `sdk` and `mcp` (`Cargo.toml`); `cli` is reserved/planned only and not a workspace member (`README.md`, `workspace.metadata.future-crates`).
- `kagi-sdk` and `kagi-mcp` both forbid unsafe Rust (`sdk/src/lib.rs`, `mcp/src/lib.rs`).
- Toolchain/format pins: Rust `1.94.0` (`rust-toolchain.toml`), `rustfmt` `max_width = 100` and `newline_style = "Unix"` (`rustfmt.toml`).

## Canonical root verification
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
CI additionally runs doc tests and workspace build with `--locked` (`.github/workflows/ci.yml`):
```bash
cargo test --doc --workspace --locked
cargo build --workspace --all-targets --locked
```
Cargo aliases (`.cargo/config.toml`): `cargo lint`, `cargo check-all`, `cargo test-all`.

## `kagi-sdk` auth/surface boundaries (do not blur)
- Official API surface: `BotToken`, `Authorization: Bot <token>`.
- Session web surface: `SessionToken`, `Cookie: kagi_session=<token>`.
- Unsupported auth/surface combinations fail before any network call.
- Route/version/surface scope is source-of-truth in `docs/endpoint-auth-version-matrix.md`.

## `kagi-mcp` v1 contract
- Transport is stdio-only (`mcp/src/main.rs` runs `serve_stdio`; `mcp/README.md` documents v1 scope).
- Capabilities are tools-only, exactly two tools: `kagi_search`, `kagi_summarize`.
- Prompts/resources are intentionally absent (documented in `mcp/README.md`, asserted in `mcp/src/tests.rs`).

## Live tests (manual-only)
- Tests are ignored by default (`#[ignore = "manual live test; run with -- --ignored"]`):
  - `sdk/tests/live_official.rs` (`KAGI_API_KEY` required)
  - `sdk/tests/live_session.rs` (`KAGI_SESSION_TOKEN` required)
- `KAGI_BASE_URL` is optional in both live tests.
- Focused runs:
```bash
KAGI_API_KEY=... cargo test -p kagi-sdk --test live_official -- --ignored
KAGI_SESSION_TOKEN=... cargo test -p kagi-sdk --test live_session -- --ignored
# optional for either: KAGI_BASE_URL=https://...
```

## Release + PR workflow constraints
- Release tag channels are only `sdk-vX.Y.Z` and `mcp-vX.Y.Z` (no bare `vX.Y.Z`) (`docs/releasing.md`, publish workflows).
- Release tag commit must already be reachable from `origin/main` (`sdk-publish.yml`, `mcp-release.yml`).
- `mcp/Cargo.toml` must keep exact SDK pin `kagi-sdk = { version = "=X.Y.Z" }`; a local `path` is allowed only when paired with the same exact version, and MCP release validates both pin shape and crates.io availability of that SDK version (`docs/releasing.md`, `.github/workflows/mcp-release.yml`).
- PR title gate: `<type>(optional-scope): <summary>` with allowed types `feat|fix|docs|chore|refactor|test|build|ci|perf|style|revert` (`.github/workflows/pr-title.yml`; regex also allows optional `!` before `:`).
