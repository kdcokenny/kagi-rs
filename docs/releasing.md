# Releasing and Workflow Policy (v1)

Maintainer/operator policy for release tags, publish automation, live tests, and workflow contracts.

## Release tag policy (dual-channel automation)

- `sdk/` (`kagi-sdk`) has its own explicit crate version in `sdk/Cargo.toml`.
- `mcp/` (`kagi-mcp`) has its own explicit crate version in `mcp/Cargo.toml` and is publishable on crates.io.
- `mcp/Cargo.toml` pins the SDK edge with an exact table-form requirement (`kagi-sdk = { version = "=X.Y.Z" }`) and may include a local path only when paired with that exact same pinned version (`kagi-sdk = { path = "../sdk", version = "=X.Y.Z" }`).
- Tags are channel-specific and non-overlapping:
  - SDK crates.io publish + GitHub Release object: `sdk-v*.*.*` → `.github/workflows/sdk-publish.yml`
  - MCP crates.io publish + GitHub Release object: `mcp-v*.*.*` → `.github/workflows/mcp-release.yml`
- Bare `v*.*.*` tags are not a release path.

## crates.io token policy (shared secret)

- Both release workflows use one repository secret: `CARGO_REGISTRY_TOKEN`.
- Bootstrap for first MCP publish:
  1. create token with both scopes (`publish-new` + `publish-update`)
  2. run first successful `mcp-vX.Y.Z` release
  3. rotate token to `publish-update` only for steady-state operation

## MCP release recovery posture

- `mcp-vX.Y.Z` publishes `kagi-mcp` to crates.io and upserts an assetless GitHub Release object.
- If crates.io publish already succeeded on a prior attempt, rerunning reconciles only the GitHub Release object (no duplicate publish).
- Never reuse a previously published MCP version/tag.

## MCP release tag helper (local pre-tag helper)

Use `scripts/mcp-release-tag.py` before creating/pushing an MCP release tag.

Safety contract enforced by the helper:

1. Reads MCP version directly from `mcp/Cargo.toml` (`name = "kagi-mcp"`, explicit `version = "X.Y.Z"`).
2. Derives only `mcp-v<version>`.
3. Requires `HEAD == origin/main`.
4. Refuses to rewrite existing semver tags.
5. Allows `--force` only for a non-`--check` safe push retry when the local tag already points to `HEAD` and origin is missing that tag.
6. Requires a clean worktree/index only for create/push actions.
7. `--check` is non-destructive and prints resolved version, derived tag, and planned action (`noop`, `push-existing`, or `create-and-push`), and still enforces the clean-tree/head-stability provenance guard when the planned action is mutating (`push-existing` or `create-and-push`). In MCP helper check mode, `push-existing` preview does not require `--force`; `--check --force` is rejected as invalid.

```bash
scripts/mcp-release-tag.py --check
scripts/mcp-release-tag.py
scripts/mcp-release-tag.py --force
```

## SDK release tag helper (local pre-tag helper)

Use `scripts/sdk-release-tag.py` before creating/pushing an SDK publish tag.

Safety contract enforced by the helper:

1. Reads SDK version directly from `sdk/Cargo.toml` (`name = "kagi-sdk"`, explicit `version = "X.Y.Z"`).
2. Derives only `sdk-v<version>`.
3. Requires `HEAD == origin/main`.
4. Refuses to rewrite existing semver tags.
5. Allows `--force` only for a safe push retry when the local tag already points to `HEAD` and origin is missing that tag.
6. Requires a clean worktree/index only for create/push actions.
7. `--check` is non-destructive and prints resolved version, derived tag, and planned action (`noop`, `push-existing`, or `create-and-push`), and still enforces the clean-tree/head-stability provenance guard when the planned action is mutating (`push-existing` or `create-and-push`).

```bash
scripts/sdk-release-tag.py --check
scripts/sdk-release-tag.py
scripts/sdk-release-tag.py --force
```

## GitHub Actions workflows (v1)

The repository uses six workflows under `.github/workflows/`:

| Workflow | Purpose |
|---|---|
| `ci.yml` | Required merge gate for formatting, linting, tests, doc-tests, and workspace build |
| `pr-title.yml` | Metadata-only PR title validation on `pull_request_target` |
| `security.yml` | PR dependency risk checks plus scheduled/mainline Rust dependency auditing |
| `live-integration.yml` | Manual SDK live integration tests routed to one protected environment |
| `sdk-publish.yml` | Tag-driven, guarded crates.io publish pipeline for `kagi-sdk` plus GitHub Release object upsert |
| `mcp-release.yml` | Tag-driven, guarded crates.io publish pipeline for `kagi-mcp` plus GitHub Release object upsert |

No separate `examples.yml` workflow is used in v1.

## Manual live-test policy (v1)

Live integration runs are intentionally manual-only via `workflow_dispatch` in `live-integration.yml`.

- Input: `target` (`official` or `session`)
- Routing: exactly one live job path per dispatch
- Triggers excluded in v1: no PR, no push, no schedule
- SDK live test targets only (MCP live tests are deferred to v2)
- Official target preflight: dispatch ref must be `main`, `sdk-v*.*.*`, or `mcp-v*.*.*` tag whose commit is reachable from `origin/main` before environment-gated jobs run
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
   - `kagi-live-official`: allow `main` and manual dispatch against `sdk-v*.*.*` and/or `mcp-v*.*.*` tags
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
| `live-integration.yml` | `workflow_dispatch` only | `contents: read` (job-scoped) | Route by `target` to exactly one environment (`kagi-live-official` or `kagi-live-session`) | Serialized concurrency per environment, hard timeouts, compile live test binaries in no-secret steps, then execute binaries in secret-bearing steps only, official target preflight enforces main-or-`sdk-v*.*.*`/`mcp-v*.*.*`-from-main before environment job, SDK live tests only |
| `sdk-publish.yml` | tag pushes `sdk-v*.*.*` only | publish job: `contents: read`; release job: `contents: write` | None | Canonical repo guard, tag commit reachable from `origin/main`, tag/version invariant check, run-attempt-aware publish decision (`published_on_first_attempt`, `published_on_retry`, `recovered_after_publish`, `conflict`), full workspace quality gates only when publish is required, publish crate via shared `CARGO_REGISTRY_TOKEN`, then upsert an assetless GitHub Release object via `GITHUB_TOKEN` |
| `mcp-release.yml` | tag pushes `mcp-v*.*.*` only | publish job: `contents: read`; release job: `contents: write` | None | Canonical repo guard, tag commit reachable from `origin/main`, tag/version invariant check, enforce exact table-form `kagi-sdk` dependency (`kagi-sdk = { version = "=X.Y.Z" }` or `kagi-sdk = { path = "../sdk", version = "=X.Y.Z" }`) and require that SDK version on crates.io, run-attempt-aware publish decision (`published_on_first_attempt`, `published_on_retry`, `recovered_after_publish`, `conflict`), full workspace quality gates only when publish is required, publish crate via shared `CARGO_REGISTRY_TOKEN`, then upsert an assetless GitHub Release object via `GITHUB_TOKEN` |
