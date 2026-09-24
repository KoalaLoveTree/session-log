# Android

The phone’s copy. The note and the sync rules are in `SPEC.md`.

SQLite. `entries` stores the note in `SPEC.md`. The row also stores `synced_at` and, while both sides do not agree, the other body. Until then, the other body is null.

While the other body is set, the phone shows both texts and a box for the one that remains. The write saves here first: that body, the other body null, `deleted_at` null, and `updated_at` set to this write. The next sync carries it, as in `SPEC.md`.

`purges` stores an id and `purged_at`. No body.

A write is saved here before it is sent to the PC. A purge of a note that has synced is kept and sent. A note created and purged here before it ever synced is not sent.

When a response entry arrives for an id in `purges`, drop that purge and store the note. When a purge arrives for a note still stored here, ask before removing it. `updated_at` equals `synced_at`: “The PC purged this. Remove it here too?” Otherwise: “The PC purged this, and this copy changed after the last sync. Remove it here too?” Accept removes the row and keeps the purge. Decline sends this copy back and removes the purge.

No Postgres on the phone.
