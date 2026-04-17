# Endpoint / Auth / Version Matrix (SDK v1 Source of Truth)

Status updated: 2026-04-17

## In-scope endpoints

| Endpoint ID | Surface | Method | Route | Version | Parser shape | Allowed credential | In SDK v1 |
|---|---|---:|---|---|---|---|---|
| `official.search` | Official API | GET | `/api/v0/search` | `v0` | JSON envelope | `BotToken` | ✅ |
| `official.enrich_web` | Official API | GET | `/api/v0/enrich/web` | `v0` | JSON envelope | `BotToken` | ✅ |
| `official.enrich_news` | Official API | GET | `/api/v0/enrich/news` | `v0` | JSON envelope | `BotToken` | ✅ |
| `official.summarize_get` | Official API | GET | `/api/v0/summarize` | `v0` | JSON envelope | `BotToken` | ✅ |
| `official.summarize_post` | Official API | POST | `/api/v0/summarize` | `v0` | JSON envelope | `BotToken` | ✅ |
| `official.fastgpt` | Official API | POST | `/api/v0/fastgpt` | `v0` | JSON envelope | `BotToken` | ✅ |
| `official.smallweb_feed` | Official API | GET | `/api/v1/smallweb/feed` | `v1` | JSON envelope | `BotToken` | ✅ |
| `session.html_search` | Session web | GET | `/html/search` | n/a | HTML | `SessionToken` | ✅ |
| `session.summary_labs_get` | Session web | GET | `/mother/summary_labs` | n/a | Stream/SSE-like | `SessionToken` | ✅ |
| `session.summary_labs_post` | Session web | POST | `/mother/summary_labs/` | n/a | Stream/SSE-like | `SessionToken` | ✅ |

## Explicitly out of scope in SDK v1

| Surface | Pattern |
|---|---|
| Official API | Any `/api/*` route not listed above |
| Session web | Any web route not listed above |

## Enforcement contract

- Unsupported auth/surface combinations fail before network calls.
- Official API 401/403 responses map to `KagiError::UnauthorizedBotToken`.
- Session web 401/403 responses map to `KagiError::InvalidSession`.
- Official API envelope failures map to `KagiError::ApiFailure`.
