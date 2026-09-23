# Android

The phone’s copy. The note and the sync rules are in `SPEC.md`.

SQLite. One table `entries`. Note columns store the note in `SPEC.md`. The row also stores the last body both sides agreed on, and the other body when both sides have changed that note. Until then, the other body is null.

A write is saved here before it is sent to the PC.

No Postgres on the phone. No screens.
