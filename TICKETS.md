# Tickets

Exactly one item in **Now**, with what / not. **Next** is the queue. **Later** is a parking lot — no dates, do not start from here unless using the app hurt. A finished item moves to `DONE.md`.

## Now

- Android sync shape in the specs. Not: the phone app, tests, or OpenAPI.
  - The phone connects. Opening the app sends notes the PC is missing and pulls notes the phone is missing.
  - Written on the phone: SQLite first, then the PC. Written on the site: Postgres, and the phone pulls it on the next open.
  - Both sides edited the same note: keep both texts. Neither side overwrites the other during sync.
  - A note only one side has syncs with no merge screen. Created time is when you wrote it, not when the sync finished.

## Next

- Implement that sync. Not: the merge screen, or tests. OpenAPI attributes on any route this adds.
- Tests for that sync. Not: new behavior, or phone UI tests.
- Merge on the phone and on the PC site. Not: an automatic winner.
  - Either side shows both texts. You write the one that remains, and the sync carries it to the other side.
  - A delete on one side and an edit on the other waits for that same step.
- While that app is open, the PC can tell the open connection to pull again. Not: the PC calling the phone, and not a sync with the app closed.
- Only the owner can open the log. Not: accounts for other people, a public host, or a new Wi-Fi password.
  - Anyone on the Wi-Fi can open the site today. The Wi-Fi password stays strong and rotated. It is not the app gate.
  - One person. The phone on the same Wi-Fi still has to get in.
  - The v1 fence and the screen spec say no login. This item is the change.
  - The gate itself is not chosen yet.

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
