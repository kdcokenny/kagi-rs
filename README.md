# kagi-rs

Rust-native tooling workspace for Kagi.

`kagi-rs` currently ships an SDK crate and an MCP crate built on top of that SDK.

## Workspace status

| Path | Crate | Status | Notes |
|---|---|---|---|
| `sdk/` | `kagi-sdk` | ✅ Available | Typed Rust SDK with explicit official-api and session-web surfaces |
| `cli/` | *(planned)* | ⏳ Planned | Reserved for a future end-user command-line interface |
| `mcp/` | `kagi-mcp` | ✅ Implemented (non-publishable) | MCP stdio server exposing `kagi_search` and `kagi_summarize`; `publish = false` in this phase |

## SDK release policy (manual-first)

- **Publish boundary in this phase**:
  - `sdk/` (`kagi-sdk`) is the only crate prepared for crates.io publication.
  - `mcp/` (`kagi-mcp`) remains workspace-only with `publish = false`.
  - `cli/` remains out of scope and is not a workspace package in this phase.
- **Tag namespace boundary**:
  - `v*.*.*` tags are reserved for the MCP GitHub release workflow.
  - SDK bookkeeping tags must use `sdk-v*.*.*` only.
- **Workflow boundary in this phase**:
  - do not repurpose `.github/workflows/release.yml`
  - do not add a dedicated SDK publish workflow yet
- **Shared versioning choice**: workspace version remains shared via `workspace.package.version = "0.1.0"` and is consumed by workspace crates.

### SDK post-publish tag helper (local)

Use `scripts/sdk-release-tag.py` only after `kagi-sdk` is already published on crates.io from the exact current clean `HEAD` snapshot on `origin/main`.

Safety contract enforced by the helper:

1. Reads the effective SDK version from `workspace.package.version` plus `sdk/Cargo.toml` (`name = "kagi-sdk"`, `version.workspace = true`).
2. Fetches and requires `HEAD == origin/main`.
3. Requires `kagi-sdk@<version>` to already exist on crates.io.
4. Derives only `sdk-v<version>` (never `v<version>`).
5. Requires a completely clean working tree and index (including untracked files) before any create/push action.
6. Refuses to rewrite existing SDK semver tags.
7. Allows `--force` only for a safe push retry of an already-correct existing local tag when origin is missing it.

Preview checks without creating or pushing tags:

```bash
scripts/sdk-release-tag.py --check
```

Create and push the SDK bookkeeping tag (post-publish):

```bash
scripts/sdk-release-tag.py
```

If local `sdk-v<version>` already exists at `HEAD` but origin is missing it, retry push only:

```bash
scripts/sdk-release-tag.py --force
```

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

## GitHub Actions workflows (v1)

The repository uses exactly five workflows under `.github/workflows/`:

| Workflow | Purpose |
|---|---|
| `ci.yml` | Required merge gate for formatting, linting, tests, doc-tests, and workspace build |
| `pr-title.yml` | Metadata-only PR title validation on `pull_request_target` |
| `security.yml` | PR dependency risk checks plus scheduled/mainline Rust dependency auditing |
| `live-integration.yml` | Manual SDK live integration tests routed to one protected environment |
| `release.yml` | Tag-driven, guarded release pipeline for `kagi-mcp` binary artifacts |

No separate `examples.yml` workflow is used in v1.

## Manual live-test policy (v1)

Live integration runs are intentionally manual-only via `workflow_dispatch` in `live-integration.yml`.

- Input: `target` (`official` or `session`)
- Routing: exactly one live job path per dispatch
- Triggers excluded in v1: no PR, no push, no schedule
- SDK live test targets only (MCP live tests are deferred to v2)
- Official target preflight: dispatch ref must be `main` or a `v*.*.*` tag whose commit is reachable from `origin/main` before environment-gated jobs run
- Secret handling: live test binaries are compiled in no-secret steps, then executed in separate secret-bearing steps

## Protected environment setup checklist (operator-facing)

Before enabling live runs, configure repository environments in GitHub:

1. Create both environments exactly:
   - `kagi-live-official`
   - `kagi-live-session`
2. Attach required reviewers (at least one reviewer per environment).
3. Set self-review policy:
   - `kagi-live-official`: **prevent self-review disabled** in v1
   - `kagi-live-session`: **prevent self-review enabled**
4. Apply deployment branch/tag restrictions:
   - `kagi-live-official`: allow `main` and manual dispatch against `v*.*.*` tags
   - `kagi-live-session`: allow `main` only
5. Load only approved environment secrets:
   - `kagi-live-official`: `KAGI_API_KEY` and optional `KAGI_BASE_URL`
   - `kagi-live-session`: `KAGI_SESSION_TOKEN` and optional `KAGI_BASE_URL`

## Final v1 workflow contract

| Workflow | Triggers | Minimal permissions | Environments / routing | Key contract rules |
|---|---|---|---|---|
| `ci.yml` | `pull_request`, `push` (`main`), `merge_group` | `contents: read` | None | Required merge gate shape, no path filters, non-release concurrency cancellation |
| `pr-title.yml` | `pull_request_target` | `pull-requests: read` | None | Metadata-only title check, no checkout, no secret access |
| `security.yml` | `pull_request`, `push` (`main`), weekly schedule | PR: `contents: read`, `pull-requests: read`; main/schedule: `contents: read` | None | PR runs dependency-review only; main/schedule run `cargo deny` and `cargo audit` |
| `live-integration.yml` | `workflow_dispatch` only | `contents: read` (job-scoped) | Route by `target` to exactly one environment (`kagi-live-official` or `kagi-live-session`) | Serialized concurrency per environment, hard timeouts, compile live test binaries in no-secret steps, then execute binaries in secret-bearing steps only, official target preflight enforces main-or-`v*.*.*`-from-main before environment job, SDK live tests only |
| `release.yml` | tag pushes `v*.*.*` only | validate/build: `contents: read`; release upload: `contents: write` | None | Canonical repo guard, tag commit must already be reachable from `origin/main`, tag version must match `workspace.package.version`, validate workspace before packaging, build exactly four `kagi-mcp` targets, publish checksums + release assets |
