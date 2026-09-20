# Tickets

Exactly one item in **Now**, with what / not. **Next** is the queue. **Later** is a parking lot — no dates, do not start from here unless using the app hurt.

## Now

- Menu: log / deleted (and room for more). Not: React Router unless we pick it.

## Next

- README for a stranger: clone URL, what it is in a few lines, one screenshot. Not: a marketing site.
- `LICENSE` (MIT unless you pick another). Not: CLA or contributor docs.
- Continue a list on Shift+Enter: if the current line is already `- ` / `* `, the next line starts with the same marker. Not: a rich-text plugin unless we pick one tomorrow.
  - Phone: Enter is newline there — same “next bullet” idea if the line is already a list item
  - Discuss: tiny JS in the textarea vs a text plugin

## Later

- Tags, search. Not: until there are enough notes to bother.
- Weekly summary once there is data
- Pretty LAN name (`session-log.lan` / `.local`): router DNS or mDNS. Not app code; not for the stranger clone path.
- More than 50 notes: pagination, load more, or another shape. Not: pick the UI in this line.
  - `GET /api/entries` and `GET /api/entries/deleted` both `LIMIT 50` today
  - Discuss later: pages vs “load more” vs by date
  - Same cap on the deleted URL

## Done

- Working files: `AGENTS.md` (four rules + v1 fence) + this list
- Tight spec: `SPEC.md` (folders, two routes, one screen, compose)
- Compose + something at `:3000` (placeholder page, `127.0.0.1` only)
- Dark page by default. Not: theme toggle, light theme, or a CSS framework
- README is the clone path; personal notes in gitignored `GROK.md`
- First real `POST`/`GET` + the page. Not: OpenAPI, CI, extra routes, or LAN bind
- Ticket loop: short plan, stop for go (`AGENTS.md`)
- Center the page; Enter submits, Shift+Enter new line
- Soft-delete an entry with confirm. Not: purge, restore UI, or showing deleted rows
- Show newlines in the list (`white-space: pre-wrap`)
- Refactor `api/src/main.rs`: qualify crate types at the use site. Not: extra crates or files
- Edit an entry. Not: history, drafts, or a second page
- Separate test vs live (two compose projects, ports 3000 / 3001). Not: a second source tree
- Lists/bullets: `- ` disc, `* ` circle. Not: a plugin or nested lists
- Restore soft-deleted entries at `/deleted`. Not: a menu or purge
- Purge a soft-deleted row (`DELETE /api/entries/{id}/purge`). Not: wipe visible entries
- Phone on the same Wi‑Fi: `:3000` on all interfaces. Not: public deploy or a pretty hostname
- Phone Enter is a newline; desktop Enter still submits. Not: drop Add/Save
- OpenAPI + Swagger at `/api/docs` (dark CSS). Not: rustfmt, clippy, or GitHub
- Local `rustfmt` / `clippy`. Not: Actions
- Rename to session log (`session-log`). Not: tags/search
- API tests (`cargo test`). Not: unit tests or Playwright
- GitHub Actions: fmt, clippy, test, build. Not: inventing origin (you added it)
- Reordered the board: menu **Now**; tags/search **Later** until there are notes
