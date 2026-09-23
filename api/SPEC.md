# API

The table and the routes. The note is in `SPEC.md`.

## Data

Postgres. One table `entries`. Columns store the note in `SPEC.md`. `created_at` defaults to `now()`.

| Column | Type |
| --- | --- |
| `id` | `TEXT` PK |
| `body` | `TEXT NOT NULL` |
| `created_at` | `TIMESTAMPTZ NOT NULL` |
| `updated_at` | `TIMESTAMPTZ NOT NULL` |
| `deleted_at` | `TIMESTAMPTZ NULL` |

No other tables.

## Routes

Responses use the entry in `SPEC.md`.

### `POST /api/entries`

Request: `{ "id": "string", "body": "string" }`

- 201 the entry. `created_at` and `updated_at` are the insert time. `deleted_at` is `null`
- 400 `{ "error": "id must be a uuid" }` if `id` is missing or not a lowercase UUID
- 400 `{ "error": "body must not be empty" }` if `body` is missing, empty, or whitespace-only
- 409 `{ "error": "id already used" }` if that `id` is already stored

### `GET /api/entries`

- 200 `{ "entries": [ entry, ... ] }`
- Visible only. Newest first. Cap 50.

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

Remove the row. Only if `deleted_at` is set.

- 204 if it was deleted
- 404 `{ "error": "not found" }` if missing or still visible

### `GET /api/openapi.json`

Generated OpenAPI document.

### `GET /api/docs`

Swagger UI for that document (dark overlay, same palette as the log).

No other endpoints.
