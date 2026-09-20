# Tickets

Exactly one item in **Now**, with what / not. **Next** is capped at four. **Later** is a parking lot — no dates, do not start from here unless using the app hurt.

## Now

- Compose + something at `:3000`. Not: real POST/GET, the full page, or extra services.

## Next

- First real `POST`/`GET` + the page.
- Phone on the same Wi‑Fi: publish `:3000` on all interfaces (not `127.0.0.1`), one README line with `http://<lan-ip>:3000`. Not: public deploy, HTTPS, extra ports, or a second hostname.

## Later

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
