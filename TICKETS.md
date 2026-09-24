# Tickets

**Now** holds the ticket in progress, with what / not, and is empty between tickets. **Next** is the queue. **Later** is a parking lot — no dates, do not start from here unless using the app hurt. A finished item moves to `DONE.md`.

## Now

## Next

- Merge on the phone and on the PC site. Not: an automatic winner.
  - Either side shows both texts. You write the one that remains, and the sync carries it to the other side.
  - A delete on one side and an edit on the other waits for that same step.
- While that app is open, the PC can tell the open connection to pull again. Not: the PC calling the phone, and not a sync with the app closed.
- Only the owner can open the log. Not: accounts for other people, a public host, or a new Wi-Fi password.
  - Anyone on the Wi-Fi can open the site today. The Wi-Fi password stays strong and rotated. It is not the app gate.
  - One person. The phone on the same Wi-Fi still has to get in.
  - The v1 fence and the screen spec say no login. This item is the change.
  - The gate itself is not chosen yet.
- Sync cases `api/tests/sync.rs` does not state yet. Not: new behavior, phone UI tests, or replacing the tests already committed.
  - A note created and purged before it ever synced is not sent, and that id can be written again.
  - A soft-delete only the PC has, and a soft-delete only the phone has, are kept and stamped.
  - A bad purge id or a bad purge time is 400. Decline of a missing row rejects a bad id, a blank body, and a bad time, and the purge stays.
  - A second purge for an id already stored leaves the first `purged_at`.

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
