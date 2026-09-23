# v1 spec

How the modules meet. Implementation is sliced in `TICKETS.md`. Do not add routes, tables, or screens that are not in these specs.

- `api/SPEC.md` — table, routes, and sync state
- `web/SPEC.md` — the screen
- `android/SPEC.md` — the phone copy

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
  src/main.rs
  migrations/         # sqlx
web/                  # Vite + React + TypeScript
  package.json
  Dockerfile
  src/App.tsx
android/SPEC.md       # phone copy; not a compose service
```

Do not add packages, crates, or routers until a file is too big to explain.

## Note

An entry is `{ "id", "body", "created_at", "updated_at", "deleted_at" }`. Times are RFC3339. `deleted_at` is `null` while the note is on the log.

| Field | Rules |
| --- | --- |
| `id` | Chosen when the note is written. A lowercase UUID (`8-4-4-4-12` hex). A stored id in any other form is replaced once with a new UUID. The other fields stay |
| `body` | trimmed; reject empty |
| `created_at` | when the note was written. Unchanged by edit, delete, restore, and sync |
| `updated_at` | set on create, edit, soft-delete, and restore. It does not choose which text remains |
| `deleted_at` | set on soft-delete; hidden from the list |

## Who connects

Same origin: the browser only talks to the web origin. The web container proxies `/api` to the api container. The phone uses that same `/api`.

## Sync

The phone opens the connection when its app opens. Compare each note to the last body both sides agreed on.

- Only one side has it: that copy is sent across. No merge screen.
- Only one side changed: that body is sent across, and it becomes the body they agree on.
- Both sides changed: each side keeps its `body` and keeps the other text beside the note.
- Both bodies are the same: that body is the one they agree on.

The other text is not a note field and is not shown.

## Compose

Three services:

| Service | Role |
| --- | --- |
| `db` | Postgres 16, healthcheck, named volume |
| `api` | Build `api/`, run migrations on start, listen internally (`8080`) |
| `web` | Build `web/`, publish **3000** (live) or **3001** (test) on all interfaces; `/` = UI |

Same compose file, two projects. Live: `.env` from `.env.example` (`COMPOSE_PROJECT_NAME=session-log`, `HOST_PORT=3000`). Test: `.env.test` from `.env.test.example` (`session-log-test`, `3001`). Each project gets its own `db-data` volume. `.env` from `.env.example`: `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB`. `DATABASE_URL` for `api` is built from those. A local default password lives in `.env.example`, not in README prose.

## Out

Extra folders for “structure,” CORS (not needed behind the proxy).
