# Tickets

Exactly one item in **Now**, with what / not. **Next** is capped at four. **Later** is a parking lot — no dates, do not start from here unless using the app hurt.

## Now

## Next

## Later

- Restore a soft-deleted entry. Not: a trash page in this item; hide deleted until then.
- Hard-delete (purge) soft-deleted rows. Not: wipe visible entries.
- Phone on the same Wi‑Fi: publish `:3000` on all interfaces (not `127.0.0.1`), one README line with `http://<lan-ip>:3000`. Not: public deploy, HTTPS, extra ports, or a second hostname.
- OpenAPI (`utoipa`), GitHub Actions, rustfmt/clippy in CI
- Tags, search, rename `lr`
- Weekly summary once there is data
- Pretty LAN name (`lr.lan` / `.local`): router DNS or mDNS. Not app code; not for the stranger clone path.

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
