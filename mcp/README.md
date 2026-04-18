# kagi-mcp

Rust MCP stdio server built on top of `kagi-sdk`.

## Scope (v1)

- Transport: **stdio only**
- Tools: exactly two
  - `kagi_search`
  - `kagi_summarize`
- Prompts: none
- Resources: none

Both tools are annotated as read-only and idempotent.

## Backend selection

`kagi-mcp` selects one backend at startup and keeps it fixed for the process lifetime.

| Env | Values | Default | Behavior |
|---|---|---|---|
| `KAGI_MCP_BACKEND` | `auto`, `official`, `session` | `auto` | Chooses which Kagi surface is used |

Credentials:

- `KAGI_API_KEY` (official API / bot-token flow)
- `KAGI_SESSION_TOKEN` (session-web flow)

Resolution rules:

1. `auto`: prefer `KAGI_API_KEY`; if missing, use `KAGI_SESSION_TOKEN`; if both missing, startup fails
2. `official`: require valid `KAGI_API_KEY`
3. `session`: require valid `KAGI_SESSION_TOKEN`

No mid-call fallback is performed.

## Tool contracts

### `kagi_search`

Input:

- `query` (required, trimmed, non-blank)
- `limit` (optional, default `5`, min `1`, max `10`)

Output:

- `results`: array of `{ title, url, snippet? }`
- `total_returned`

### `kagi_summarize`

Input (xor):

- `url` **or** `text` (exactly one)
- `url` must be absolute HTTP(S)
- `text` preserves caller whitespace and must be `<= 50_000` UTF-8 bytes

Output:

- `markdown`
- optional `text`
- optional `source_url`

Unknown input fields are rejected.

## Run

```bash
# auto mode (preferred: API key)
KAGI_API_KEY=... cargo run -p kagi-mcp

# force session mode
KAGI_MCP_BACKEND=session KAGI_SESSION_TOKEN=... cargo run -p kagi-mcp
```

## Development

From workspace root:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
