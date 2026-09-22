# Tickets

Exactly one item in **Now**, with what / not. **Next** is the queue. **Later** is a parking lot — no dates, do not start from here unless using the app hurt.

## Now

- Split the spec into module specs before any Android behavior. Not: sync rules, the phone app, tests, or OpenAPI.
  - Shared note fields and who connects stay in `SPEC.md`. Existing routes move to `api/SPEC.md`, the screen to `web/SPEC.md`.
  - `android/SPEC.md` appears with the sync slice, which is the next ticket.

## Next

- Android sync shape in the specs. Not: the phone app, tests, or OpenAPI.
  - The phone connects. Opening the app sends notes the PC is missing and pulls notes the phone is missing.
  - Written on the phone: SQLite first, then the PC. Written on the site: Postgres, and the phone pulls it on the next open.
  - Both sides edited the same note: keep both texts. Neither side overwrites the other during sync.
  - A note only one side has syncs with no merge screen. Created time is when you wrote it, not when the sync finished.
- Implement that sync. Not: the merge screen, or tests. OpenAPI attributes on any route this adds.
- Tests for that sync. Not: new behavior, or phone UI tests.
- Merge on the phone and on the PC site. Not: an automatic winner.
  - Either side shows both texts. You write the one that remains, and the sync carries it to the other side.
  - A delete on one side and an edit on the other waits for that same step.
- While that app is open, the PC can tell the open connection to pull again. Not: the PC calling the phone, and not a sync with the app closed.

## Later

- The app syncs on Wi-Fi without being opened. Not: the first phone version. The phone still starts the connection.
- Menu: log / deleted, and room for more lists. Not: React Router unless we pick it.
- Mentor questions and dreams as their own lists. Not: mixing them into the session log.
  - Text and a time. Same id and `updated_at` as a log note, so the backup includes them from the first row
  - Hang off the menu above
- Anime watched, as its own list. Not: titles in the session log, or the browsing layout.
  - A title plus a status. Same id and `updated_at`
  - The look waits until the plain list is annoying to scan
- Tags, search. Not: until there are enough notes to bother.
- Weekly summary once there is data
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
- Reordered the board: README **Now**; LICENSE, continue-list, weekday **Next**; menu **Later** until `/deleted` hurts
- README for a stranger: clone URL, a few lines, one screenshot. Not: a marketing site
- `LICENSE` (MIT). Not: CLA or contributor docs
- Continue a list on Shift+Enter (`- ` / `* `). Empty marker leaves the list. Not: a plugin
- Day of week on the displayed date. Not: relative time, timezone picker, or changing stored `created_at`
- Pretty LAN name `session-log.local` (Avahi, this machine). Not: app code, README, or router DNS
- Autostart live stack on boot (user systemd `session-log`, linger). Not: test stack, app code, or README
- README: Linux boot unit + linger. Not: shipping the machine-local unit, test stack, or Avahi
- 24-hour time on displayed dates. Not: timezone picker, relative time, or changing stored `created_at`
- A note a second copy can hold: writer-chosen id and `updated_at`. Not: the phone app, a sync feed, or a public host
