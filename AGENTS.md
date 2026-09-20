# lr

Before changing code, read `GROK.md`, `README.md`, and `SPEC.md`.

## Contract

1. One ticket at a time. Say what / not.
2. Model writes the patch. Human reviews it like a PR.
3. Run it. Use it. Next ticket comes from that.
4. If the human cannot explain a file, it is not yours yet — delete or rewrite.

Do not paste “build the app” and accept a 40-file dump.

## v1 fence

In: one page (text box, submit, recent list), one table (`id`, `body`, `created_at`), `POST /api/entries` and `GET /api/entries`, `docker compose up` → `http://localhost:3000`.

Out: tags, search, auth, public deploy, LLM features, PHP, scrapers, GraphQL, extra features. Localhost is enough to run. GitHub is the public surface.

Work items live in `TICKETS.md`. Do not start a ticket that is not **Now**.
