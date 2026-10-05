# Android

The phone’s copy. The note and the sync rules are in `SPEC.md`.

SQLite. `entries` stores the note in `SPEC.md`. The row also stores `synced_at` and, while both sides do not agree, the other body. Until then, the other body is null.

While the other body is set, the phone shows both texts and a box for the one that remains. The write saves here first: that body, the other body null, `deleted_at` null, and `updated_at` set to this write. The next sync carries it, as in `SPEC.md`.

`purges` stores an id and `purged_at`. No body.

A write is saved here before it is sent to the PC. A purge of a note that has synced is kept and sent. A note created and purged here before it ever synced is not sent.

When a response entry arrives for an id in `purges`, drop that purge and store the note. When a purge arrives for a note still stored here, ask before removing it. `updated_at` equals `synced_at`: “The PC purged this. Remove it here too?” Otherwise: “The PC purged this, and this copy changed after the last sync. Remove it here too?” Accept removes the row and keeps the purge. Decline sends this copy back and removes the purge.

No Postgres on the phone.

## Reminders

A schedule on the phone. Not a note. Not synced. This app does not write notes and does not open the sync connection. The phone copy above is unchanged.

Monday through Sunday. Each day is off, or on at one local time on a 24-hour clock. An on day comes due every week at that time.

The occurrence stays due until `GET /api/entries` includes a note whose `created_at` is after the time it came due. While the PC cannot be reached, it stays due. While one is due, the phone checks that list about once a minute. The open screen lists every due occurrence.

Dismissing the banner leaves it due, and the banner stays down until the next time a reminder comes due. A tap opens the stored log address in the browser.

The address starts as `http://session-log.local:3000`. The owner can change it. The LAN address is `http://<lan-ip>:3000` in `SPEC.md`.

Later applies the saved duration to this occurrence only. The owner sets that duration. Until one is set, Later asks for the time. When asks for a 24-hour clock time for this occurrence. A time still ahead today waits until then. A time already passed waits until that time tomorrow. The weekday's time stays. A note that clears the occurrence also drops a wait that has not fired yet.
