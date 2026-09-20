# Tickets

Exactly one item in **Now**, with what / not. **Next** is the queue. **Later** is a parking lot — no dates, do not start from here unless using the app hurt.

## Now

- OpenAPI (`utoipa`), GitHub Actions, rustfmt/clippy in CI

## Next

- Tags, search, rename `lr`
- Weekly summary once there is data
- Pretty LAN name (`lr.lan` / `.local`): router DNS or mDNS. Not app code; not for the stranger clone path.

## Later

- A real menu (log / deleted / whatever comes next). Not: React Router unless we pick it; we already have `/` and `/deleted` with one link each.
- More than 50 notes: pagination, load more, or another shape. Not: pick the UI in this line.
  - `GET /api/entries` and `GET /api/entries/deleted` both `LIMIT 50` today
  - Discuss later: pages vs “load more” vs by date
  - Same cap on the deleted URL

## Done

- Working files: `AGENTS.md` (four rules + v1 fence) + this list
- User skill `ticket` (`~/.grok/skills/ticket/SKILL.md`)
- User skill `commit` (`~/.grok/skills/commit/SKILL.md`): `/commit` after accept; push only if `origin` exists
- Tight spec: `SPEC.md` (folders, two routes, one screen, compose)
- User skill `status` (`~/.grok/skills/status/SKILL.md`): `/status` leftover commit/push check
- Compose + something at `:3000` (placeholder page, `127.0.0.1` only)
- Dark page by default. Not: theme toggle, light theme, or a CSS framework
- README is the clone path; personal notes in gitignored `GROK.md`
- First real `POST`/`GET` + the page. Not: OpenAPI, CI, extra routes, or LAN bind
- Ticket loop: short plan, stop for go (`ticket` skill + `AGENTS.md`)
- Center the page; Enter submits, Shift+Enter new line
- User skill `task` (`~/.grok/skills/task/SKILL.md`): `/task` parks a discussion on this board; nudge once in chat
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
