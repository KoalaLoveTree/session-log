# v1 spec

Target for the session log. Implementation is sliced in `TICKETS.md`. Do not add routes, tables, or screens that are not here.

## Run

```bash
docker compose up --build
```

Open `http://localhost:3000`. Phone on the same Wi‑Fi: `http://<lan-ip>:3000`. No other ports for the stranger.

## Folders

```
compose.yaml          # db + api + web
.env.example          # copied to .env for compose; no secrets in git
api/                  # Rust, axum, sqlx
  Cargo.toml
  Dockerfile
  src/main.rs         # as few files as you can still explain
  migrations/         # sqlx
web/                  # Vite + React + TypeScript
  package.json
  Dockerfile
  src/App.tsx         # the one screen
```

Do not add packages, crates, or routers until a file is too big to explain.

## Web

Vite + React + TypeScript. One `App.tsx`, no React Router, no UI kit, no CSS framework — a single CSS file is enough. Dark warm background, muted text (reading-mode, not near-white); no light theme or toggle in v1.

The `web` image: `npm` build, then **nginx** serves `dist/` on port 80 (published as **3000**) and reverse-proxies `/api` to `api:8080`.

No other frontend libraries in v1 unless a ticket names them.

## Data

Postgres. One table `entries`:

| Column | Type | Notes |
| --- | --- | --- |
| `id` | `BIGSERIAL` PK | |
| `body` | `TEXT NOT NULL` | trimmed; reject empty |
| `created_at` | `TIMESTAMPTZ NOT NULL` | default `now()` |
| `deleted_at` | `TIMESTAMPTZ NULL` | set on soft-delete; hidden from the list |

No other tables.

## Routes

Same origin: the browser only talks to `:3000`. The web container proxies `/api` to the api container.

### `POST /api/entries`

Request: `{ "body": "string" }`

- 201 `{ "id": number, "body": string, "created_at": "<RFC3339>" }`
- 400 `{ "error": "body must not be empty" }` if missing, empty, or whitespace-only

### `GET /api/entries`

- 200 `{ "entries": [ { "id", "body", "created_at" }, ... ] }`
- Visible only. Newest first. Cap 50.

### `GET /api/entries/deleted`

- 200 `{ "entries": [ ... ] }` — soft-deleted only, newest `deleted_at` first, cap 50

### `PUT /api/entries/{id}`

Request: `{ "body": "string" }`

- 200 `{ "id", "body", "created_at" }` — `created_at` unchanged
- 400 `{ "error": "body must not be empty" }` if missing, empty, or whitespace-only
- 404 `{ "error": "not found" }` if missing or already deleted

### `DELETE /api/entries/{id}`

Soft-delete: set `deleted_at`. Do not remove the row.

- 204 if it was visible
- 404 `{ "error": "not found" }` if missing or already deleted

### `POST /api/entries/{id}/restore`

Clear `deleted_at`.

- 200 `{ "id", "body", "created_at" }`
- 404 `{ "error": "not found" }` if missing or not deleted

### `DELETE /api/entries/{id}/purge`

Remove the row. Only if `deleted_at` is set.

- 204 if it was deleted
- 404 `{ "error": "not found" }` if missing or still visible

No other endpoints.

## Screen

Two URLs, one `App.tsx`, no React Router. nginx `try_files` serves `index.html` for both.

- `/` — text area, list, edit, delete
- `/deleted` — `GET /api/entries/deleted`, Restore → `POST /api/entries/{id}/restore`, Purge → `DELETE /api/entries/{id}/purge` (confirm). Empty: `No deleted entries.`
- A single link each way. No menu yet.
- Submit → `POST /api/entries`, then reload the list
- List: `created_at` and `body` for each entry from `GET /api/entries`
- Empty: `No entries yet.`
- Delete on a row → confirm “Are you sure?” → `DELETE /api/entries/{id}`, then reload the list
- Edit on a row → textarea + Save / Cancel → `PUT /api/entries/{id}`, then reload the list
- Lines starting with `- ` show as disc bullets; `* ` as circle. Stored text stays plain. Edit shows the raw `- `/`* `.

No tags, search, login, or a nav menu.

## Compose

Three services:

| Service | Role |
| --- | --- |
| `db` | Postgres 16, healthcheck, named volume |
| `api` | Build `api/`, run migrations on start, listen internally (`8080`) |
| `web` | Build `web/`, publish **3000** (live) or **3001** (test) on all interfaces; `/` = UI, `/api` = proxy to `api` |

Same compose file, two projects. Live: `.env` from `.env.example` (`COMPOSE_PROJECT_NAME=lr`, `HOST_PORT=3000`). Test: `.env.test` from `.env.test.example` (`lr-test`, `3001`). Each project gets its own `db-data` volume. `.env` from `.env.example`: `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB`. `DATABASE_URL` for `api` is built from those. A local default password lives in `.env.example`, not in README prose.

## Out

OpenAPI, CI, extra folders for “structure,” a second page, CORS (not needed behind the proxy).
