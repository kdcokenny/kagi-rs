# kagi-sdk

Rust SDK for Kagi with two explicit protocol surfaces:

- `official_api()` → Bot-token official API routes
- `session_web()` → session-cookie web routes

Examples:

```bash
cargo run -p kagi-sdk --example bot_token
cargo run -p kagi-sdk --example session_token
```
