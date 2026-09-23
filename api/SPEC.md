# API

The table, the routes, and sync state. The note and the sync rules are in `SPEC.md`.

## Data

Postgres. Note columns on `entries` store the note in `SPEC.md`. `created_at` defaults to `now()`.

| Column | Type |
| --- | --- |
| `id` | `TEXT` PK |
| `body` | `TEXT NOT NULL` |
| `created_at` | `TIMESTAMPTZ NOT NULL` |
| `updated_at` | `TIMESTAMPTZ NOT NULL` |
| `deleted_at` | `TIMESTAMPTZ NULL` |
| `synced_at` | `TIMESTAMPTZ NULL` |
| `other_body` | `TEXT NULL` |
| `declined_purged_at` | `TIMESTAMPTZ NULL` |

`synced_at`, `other_body`, and `declined_purged_at` are sync state, not note fields. `other_body` is set only while both sides are newer and the texts differ. `declined_purged_at` is the purge time a decline already answered. The entry routes do not return them.

`purges` is one row per purged id. No body.

| Column | Type |
| --- | --- |
| `id` | `TEXT` PK |
| `purged_at` | `TIMESTAMPTZ NOT NULL` |

## Sync state

The rules are in `SPEC.md`. A living entry whose id is also in `purges` is a purge waiting for an answer. Accept leaves the `purges` row and removes the entry. Decline removes the `purges` row.

## Routes

Responses use the entry in `SPEC.md`.

### `POST /api/entries`

Request: `{ "id": "string", "body": "string" }`

- 201 the entry. `created_at` and `updated_at` are the insert time. `deleted_at` is `null`
- 400 `{ "error": "id must be a uuid" }` if `id` is missing or not a lowercase UUID
- 400 `{ "error": "body must not be empty" }` if `body` is missing, empty, or whitespace-only
- 409 `{ "error": "id already used" }` if that `id` is already an entry or a purge

### `GET /api/entries`

- 200 `{ "entries": [ entry, ... ] }`
- Visible only. Newest first. Cap 50. Not the phone’s sync pull.

### `GET /api/entries/deleted`

- 200 `{ "entries": [ entry, ... ] }` — soft-deleted only, newest `deleted_at` first, cap 50

### `PUT /api/entries/{id}`

Request: `{ "body": "string" }`

- 200 the entry — `created_at` unchanged, `updated_at` set to this write
- 400 `{ "error": "body must not be empty" }` if missing, empty, or whitespace-only
- 404 `{ "error": "not found" }` if missing or already deleted

`id` in the path is the stored UUID.

### `DELETE /api/entries/{id}`

Soft-delete: set `deleted_at` and `updated_at`. Do not remove the row.

- 204 if it was visible
- 404 `{ "error": "not found" }` if missing or already deleted

### `POST /api/entries/{id}/restore`

Clear `deleted_at`. Set `updated_at`.

- 200 the entry
- 404 `{ "error": "not found" }` if missing or not deleted

### `DELETE /api/entries/{id}/purge`

Remove the row. Only if `deleted_at` is set. If `synced_at` is set, store the id and the purge time in `purges`. An id already there stays. If `synced_at` is null, do not store one.

- 204 if it was deleted
- 404 `{ "error": "not found" }` if missing or still visible

### `POST /api/sync`

The phone sends the notes it still has, and the purges it has. A sync entry is the note plus `synced_at` and `other_body`.

Request: `{ "entries": [ sync entry, ... ], "purges": [ { "id", "purged_at" } ] }`

- 200 `{ "entries": [ sync entry, ... ], "purges": [ { "id", "purged_at" } ] }`
- `entries` are notes the phone should write. A note that is already the agreed version is left out. `purges` are the ones the phone should ask about: this request still has that note, and a purge is stored for it
- A purge in this request is stored and does not remove a living entry. An id already in `purges` is not inserted
- A purge whose `purged_at` equals that entry’s `declined_purged_at` is not stored. The entry is included in the response. Any other `purged_at` is stored and `declined_purged_at` is cleared
- 400 `{ "error": "id must be a uuid" }` if an `id` is missing or not a lowercase UUID
- 400 `{ "error": "body must not be empty" }` if a note `body` is missing, empty, or whitespace-only

### `GET /api/purges/pending`

- 200 `{ "purges": [ { "id", "purged_at", "changed" } ] }` — an id that is both an entry and a purge. No body
- `changed` is true when `synced_at` is null or `updated_at` is after `synced_at`

### `POST /api/purges/{id}/accept`

Remove the entry if it exists. Keep the purge.

- 204
- 404 `{ "error": "not found" }` if that id is not in `purges`

### `POST /api/purges/{id}/decline`

Remove the purge. If the entry exists, keep the stored row. If it does not, the request is the sync entry: insert it, with the supplied `created_at`. Set `declined_purged_at` to the removed purge’s `purged_at`.

- 200 the entry
- 400 `{ "error": "id must be a uuid" }` if the entry is missing and `id` is not a lowercase UUID
- 400 `{ "error": "body must not be empty" }` if the entry is missing and `body` is missing, empty, or whitespace-only
- 404 `{ "error": "not found" }` if that id is not in `purges`

### `GET /api/openapi.json`

Generated OpenAPI document.

### `GET /api/docs`

Swagger UI for that document (dark overlay, same palette as the log).
