# lr

Before changing code, read `README.md` and `SPEC.md`. If `GROK.md` is present, read it too (local notes, not in git).

## Contract

1. One ticket at a time. Say what / not.
2. Short plan (files, how, edges, not). Stop for go, unless they already said go. Then write the patch. Human reviews it like a PR.
3. Run it. Use it. Next ticket comes from that.
4. If the human cannot explain a file, it is not yours yet — delete or rewrite.

Do not paste “build the app” and accept a 40-file dump.

## v1 fence

In: one page (text box, submit, recent list), one table (`id`, `body`, `created_at`), `POST /api/entries` and `GET /api/entries`, `docker compose up` → `http://localhost:3000`.

Out: tags, search, auth, public deploy, LLM features, PHP, scrapers, GraphQL, extra features. Localhost is enough to run. GitHub is the public surface.

Work items live in `TICKETS.md`. Do not start a ticket that is not **Now**.
If a side idea shows up that is not the current ticket, mention `/task` once. Do not write the board until they run it.
