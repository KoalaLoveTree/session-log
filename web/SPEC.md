# Web

The screen. The note and who connects are in `SPEC.md`.

Vite + React + TypeScript. One `App.tsx`, no React Router, no UI kit, no CSS framework — a single CSS file is enough. Dark warm background, muted text (reading-mode, not near-white); no light theme or toggle in v1.

The `web` image: `npm` build, then **nginx** serves `dist/` on port 80. The `/api` proxy is in `SPEC.md`.

No other frontend libraries in v1 unless a ticket names them.

## Screen

Two URLs, one `App.tsx`, no React Router. nginx `try_files` serves `index.html` for both.

- `/` — text area, list, edit, delete
- `/deleted` — `GET /api/entries/deleted`, Restore → `POST /api/entries/{id}/restore`, Purge → `DELETE /api/entries/{id}/purge` (confirm). Empty: `No deleted entries.`
- A single link each way. No menu yet.
- Submit → the page chooses a lowercase UUID and `POST /api/entries` with that `id` and the `body`, then reloads the list. The id is built with `crypto.getRandomValues`, so Add works on the phone over plain `http://`. Desktop: Enter submits, Shift+Enter newline. Phone (coarse pointer): Enter is a newline; submit with Add / Save. If that newline is on a line that already starts with `- ` or `* `, the next line starts with the same marker; an empty marker line leaves the list.
- List: `created_at` (local date/time with weekday, 24-hour clock) and `body` for each entry from `GET /api/entries`. `updated_at` and `deleted_at` are not shown. The page does not keep its own copy of the log.
- Empty: `No entries yet.`
- Delete on a row → confirm “Are you sure?” → `DELETE /api/entries/{id}`, then reload the list
- An id from `GET /api/purges/pending` that is on this page → ask. `changed` is false: “The phone purged this. Remove it here too?” `changed` is true: “The phone purged this, and this copy changed after the last sync. Remove it here too?” Confirm → `POST /api/purges/{id}/accept`. Cancel → `POST /api/purges/{id}/decline`. Then reload the list
- Edit on a row → textarea + Save / Cancel → `PUT /api/entries/{id}`, then reload the list
- Lines starting with `- ` show as disc bullets; `* ` as circle. Stored text stays plain. Edit shows the raw `- `/`* `.

No tags, search, login, or a nav menu.
