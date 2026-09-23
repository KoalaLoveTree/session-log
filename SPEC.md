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
| `updated_at` | set on create, edit, soft-delete, restore, and an agreed sync. It does not choose between two parallel edits |
| `deleted_at` | set on soft-delete; hidden from the list |

## Who connects

Same origin: the browser only talks to the web origin. The web container proxies `/api` to the api container. The phone uses that same `/api`.

## Sync

The phone opens the connection when its app opens. Each note remembers `synced_at`: the PC clock time of the last sync that agreed on it. `synced_at` is null until then. A copy is still that version when `updated_at` equals `synced_at`. A local edit sets `updated_at` after `synced_at`. The phone does that when its own clock is behind.

- Only one side has it, and its id is not a stored purge: that copy is sent across. No merge screen.
- Only one side is newer than `synced_at`: that copy is kept, including a soft-delete. Both sides then store the same `updated_at` and `synced_at`, from the PC clock. The old text is not kept.
- Both sides are newer and the texts differ: each side keeps its `body` and keeps the other text beside the note. `synced_at` stays. The merge screen is later.
- Both sides are newer and the texts are the same: that text is the one they agree on.
- Both sides have it and `synced_at` is null: the same text is the agreement. Two texts are kept beside each other.

The other text is not a note field and is not shown.

A note created and purged before it ever synced is not sent. A purge after a sync stores the id and the time, with no body. While that id is stored, the note is not written across. The side that still has the note asks before it removes its copy. Accept removes the row and leaves the id, so it is not written again. Decline keeps the surviving copy, sends it back, and removes the purge. The same purge sent again returns that copy and does not ask again. A later purge asks again. The side that still has the body supplies it.

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
