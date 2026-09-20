# lr

Working name. Personal session log — a small web app you actually open, not another scraper that dies when you switch games.

This repo is the **decision log + later the code**. It exists so the idea does not get postponed into nothing.

## Why

Camp and old bots taught Rust. They did not produce something used on a Tuesday.

This one should:

- Get used after real sessions (games, rustcamp, whatever is going on)
- Be built **with** AI, the way work looks now
- Stay small enough that features come from friction, not from a tutorial backlog

Streaming is **out** unless it comes back in life. Do not design for it.

## v1 (when we start)

One evening. Then use it for a few days before adding anything else.

- One page: text box, submit, list of recent entries
- One table: `id`, `body`, `created_at`
- Two endpoints: `POST /api/entries`, `GET /api/entries`
- `docker compose up` → `http://localhost:3000`
- Habit: close a session → add one line. If that does not happen, change the product, not the CSS.

No tags, search, auth, public deploy, or LLM features in v1.

## Stack (provisional)

Chosen to match rustcamp backend + common job tooling. Revisit after life review if the product shifts.

| Layer | Choice |
| --- | --- |
| API | Rust, `axum`, REST |
| Contract | OpenAPI (`utoipa`) once v1 works |
| DB | Postgres in Docker, `sqlx` migrations |
| UI | Thin TypeScript (Vite + React) |
| Run | `docker compose` |
| Hygiene | `rustfmt`, `clippy`, tests, GitHub Actions |

Localhost is enough. User zero is you.

## Out of scope (until we feel the lack)

- Discord bots, game API scrapers
- GraphQL, gRPC, Kubernetes
- Multi-user SaaS, OAuth
- “AI-powered notes” as the product (a weekly summary can come later, when there is data)
- Perfect design system

## How we work with AI

Full practice guide later (after life review). Until then, this is the contract:

1. One ticket at a time. Say what / not.
2. Model writes the patch. You review it like a PR.
3. Run it. Use it. Next ticket comes from that.
4. If you cannot explain a file, it is not yours yet — delete or rewrite.

Do not paste “build the app” and accept a 40-file dump.

## Resume later

1. Read this file. Life review may change v1; update this README first if it does.
2. Then a tight spec (folders, two routes, one screen, compose) and implement only that.

Open questions for after life review: is “session log” still the thing you will open? Any daily loop we missed? Keep the name `lr`?
