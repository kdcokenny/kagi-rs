# kagi-mcp

Agent-oriented MCP server for Kagi search and summarization, built on `kagi-sdk`.

> [!IMPORTANT]
> **v1 capability scope**
>
> - **Transport:** stdio only
> - **Capabilities:** tools only
> - **Tools:** `kagi_search`, `kagi_summarize`
> - **Prompts:** none
> - **Resources:** none

Both tools are read-only and idempotent.

## Quickstart (canonical)

Run the server from repository root:

```bash
KAGI_API_KEY=... cargo run -p kagi-mcp
```

## Host support matrix

| Host | Config surface / location | Transport used here | Verification | Fetch / conflict note |
|---|---|---|---|---|
| Claude Code | `~/.claude.json` → `mcpServers.<name>` (or `/mcp` / `claude mcp add`) | `stdio` | `/mcp`, `claude mcp list`, `claude mcp get kagi` | Use built-in fetch tool first if available |
| Codex | Codex MCP registry (managed by `codex mcp add`; stored in `~/.codex/config.toml`) | `stdio` | `/mcp` | Use `--env` on add; avoid duplicate search providers |
| OpenCode | Project `opencode.json(c)` or global `~/.config/opencode/opencode.json(c)` under `mcp.<name>` | `stdio` | `opencode mcp list`, `opencode mcp debug kagi` | Prefer built-in fetch/read before adding fetch MCP |

## Host setup

The snippets below target host-registered launches (often global) and therefore pin an absolute manifest path so startup does not depend on current working directory.

Replace `/absolute/path/to/kagi-rs/Cargo.toml` with your local checkout path.

Optional for long-lived global setups: `cargo install kagi-mcp` and configure host command as `kagi-mcp` to avoid repo-path dependency.

### Claude Code (native MCP)

Config surface/location: `~/.claude.json` under `mcpServers` (or the equivalent entry created via `/mcp` / `claude mcp add`).

Use this stdio server entry:

```json
{
  "mcpServers": {
    "kagi": {
      "type": "stdio",
      "command": "cargo",
      "args": ["run", "--manifest-path", "/absolute/path/to/kagi-rs/Cargo.toml", "-p", "kagi-mcp"],
      "env": {
        "KAGI_API_KEY": "YOUR_KAGI_API_KEY"
      }
    }
  }
}
```

For session-web backend, replace env with:

```json
"env": {
  "KAGI_MCP_BACKEND": "session",
  "KAGI_SESSION_TOKEN": "YOUR_KAGI_SESSION_TOKEN"
}
```

Verify:

```text
/mcp
```

```bash
claude mcp list
claude mcp get kagi
```

Caveat: keep this entry `stdio`; `kagi-mcp` v1 is not an HTTP MCP server.

### Codex (native MCP)

Config surface/location: Codex MCP registry, managed by CLI and persisted in `~/.codex/config.toml`.

Register with env embedded in the MCP entry:

```bash
codex mcp add kagi --env KAGI_API_KEY=YOUR_KAGI_API_KEY -- cargo run --manifest-path /absolute/path/to/kagi-rs/Cargo.toml -p kagi-mcp
```

Session-web variant:

```bash
codex mcp add kagi --env KAGI_MCP_BACKEND=session --env KAGI_SESSION_TOKEN=YOUR_KAGI_SESSION_TOKEN -- cargo run --manifest-path /absolute/path/to/kagi-rs/Cargo.toml -p kagi-mcp
```

Verify:

```text
/mcp
```

Caveat: use stdio registration (`-- <COMMAND>`). Do not use `--url` for `kagi-mcp` v1.

### OpenCode (native MCP)

Config surface/location: project `opencode.json` / `opencode.jsonc` (or global `~/.config/opencode/opencode.json(c)`).

Use this local MCP entry:

```json
{
  "mcp": {
    "kagi": {
      "type": "local",
      "command": ["cargo", "run", "--manifest-path", "/absolute/path/to/kagi-rs/Cargo.toml", "-p", "kagi-mcp"],
      "environment": {
        "KAGI_API_KEY": "YOUR_KAGI_API_KEY"
      },
      "enabled": true
    }
  }
}
```

Session-web backend variant:

```json
"environment": {
  "KAGI_MCP_BACKEND": "session",
  "KAGI_SESSION_TOKEN": "YOUR_KAGI_SESSION_TOKEN"
}
```

Verify:

```bash
opencode mcp list
opencode mcp debug kagi
```

Caveat: keep this as `type: "local"` stdio; do not configure `kagi-mcp` v1 as remote HTTP MCP.

## Tool map (and fetch pairing)

| Tool | Best for | Not for |
|---|---|---|
| `kagi_search` | Fresh discovery and ranked candidate URLs | Full-page content retrieval |
| Companion fetch/read tool | Fetching and extracting page body from URLs | Replacing Kagi ranking or query expansion |
| `kagi_summarize` | Summarizing a URL or provided text into concise markdown | Acting as a crawler |

### Fetch pairing rules

1. If your host already has a built-in fetch/read tool, use that first.
2. If not, pair `kagi-mcp` with a lightweight fetch MCP server.
3. If you use Jina AI as the fetch layer, **narrow or disable its search capability** so it does not overlap with `kagi_search`.

## Short agent rules

- Use `kagi_search` to discover sources.
- Fetch/read source URLs before reasoning over details.
- Use `kagi_summarize` for synthesis or compression.
- Avoid duplicate search providers in the same toolchain.

## Backend and auth selection

`kagi-mcp` selects one backend at startup for the process lifetime.

| Env var | Values | Default | Behavior |
|---|---|---|---|
| `KAGI_MCP_BACKEND` | `auto`, `official`, `session` | `auto` | Chooses API surface |
| `KAGI_API_KEY` | non-empty token | n/a | Official API auth |
| `KAGI_SESSION_TOKEN` | non-empty token | n/a | Session-web auth |

Resolution rules:

1. `auto`: prefer `KAGI_API_KEY`; else `KAGI_SESSION_TOKEN`; else startup error
2. `official`: requires valid `KAGI_API_KEY`
3. `session`: requires valid `KAGI_SESSION_TOKEN`

No mid-call fallback is performed.

Session mode from shell:

```bash
KAGI_MCP_BACKEND=session KAGI_SESSION_TOKEN=... cargo run -p kagi-mcp
```

## Example workflow (search → fetch/read → summarize)

1. Call `kagi_search` with a focused query to get candidate URLs.
2. Fetch top URLs with your host fetch tool (or paired fetch MCP/Jina fetch path).
3. Call `kagi_summarize` with either:
   - `url` for direct source summarization, or
   - `text` for summarizing fetched excerpts.

## Verification

From repository root:

```bash
cargo test -p kagi-mcp
```

Then verify host registration using each host's MCP inspection command(s) above.

## Troubleshooting

- **Startup error: missing credentials**
  - Set `KAGI_API_KEY` (official) or `KAGI_SESSION_TOKEN` (session).
- **Backend mismatch errors**
  - Align `KAGI_MCP_BACKEND` with the credential type in use.
- **Tool overlap/noisy retrieval**
  - Keep `kagi_search` as the primary search tool; narrow/disable search in companion providers (especially Jina AI).
- **No tools visible in host**
  - Re-check host MCP registration and run host verification commands.

## Development notes

From workspace root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

For release/workflow policy, see [`../docs/releasing.md`](../docs/releasing.md).
