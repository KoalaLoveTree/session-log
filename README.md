# lr

Working name. Personal session log — a small web app you actually open, not another scraper that dies when you switch games.

This repo is the **decision log + later the code**. It exists so the idea does not get postponed into nothing.

## Why

Camp and old bots taught Rust. They did not produce something used on a Tuesday. Job search starts in ~4 weeks; this has to be a public GitHub artifact, not a private toy.

This one should:

- Get used after real evenings (camp is 11:00–19:00; then this, then games)
- Be built **with** AI, the way work looks now
- Stay small enough that features come from friction, not from a tutorial backlog
- Clone + README for a stranger / mentor, with no walkthrough from you

Streaming is **out** unless it comes back in life. Do not design for it. Do not expand scope.

## v1

One evening to get it running. Then use it for a few days before adding anything else.

Product (you):

- One page: text box, submit, list of recent entries
- One table: `id`, `body`, `created_at`
- Two endpoints: `POST /api/entries`, `GET /api/entries`
- `docker compose up` → `http://localhost:3000`
- Habit: close an evening → add one line. If that does not happen, change the product, not the CSS.

Artifact (mentor):

- README a stranger can follow: clone, compose, open, post a line
- No private setup lore. If you needed a note to run it, it belongs in the README
- `rustfmt`, `clippy`, tests, GitHub Actions — enough that a first look is not “it doesn’t build”

**Done when** you have logged real evenings in it, and a stranger can clone + compose without you.

Shape: `SPEC.md`.

No tags, search, auth, public deploy, or LLM features in v1. Localhost is enough to *run*. GitHub is the public surface.

## Stack

Chosen to match rustcamp backend + common job tooling. PHP would ship faster; do not use it — the artifact has to be Rust.

| Layer | Choice |
| --- | --- |
| API | Rust, `axum`, REST |
| Contract | OpenAPI (`utoipa`) once v1 works |
| DB | Postgres in Docker, `sqlx` migrations |
| UI | Thin TypeScript (Vite + React) |
| Run | `docker compose` |
| Hygiene | `rustfmt`, `clippy`, tests, GitHub Actions |

User zero is you. User one is the mentor who clones.

## Out of scope (until we feel the lack)

- Discord bots, game API scrapers
- GraphQL, gRPC, Kubernetes
- Multi-user SaaS, OAuth, public deploy
- “AI-powered notes” as the product (a weekly summary can come later, when there is data)
- Perfect design system
- Rewriting this in PHP

## How we work with AI

1. One ticket at a time. Say what / not.
2. Model writes the patch. You review it like a PR.
3. Run it. Use it. Next ticket comes from that.
4. If you cannot explain a file, it is not yours yet — delete or rewrite.

Do not paste “build the app” and accept a 40-file dump.

## Next

Work items live in `TICKETS.md`. Keep the name `lr` until a stranger would not know what the repo is. Do not bikeshed it now.
