# lr

Personal session log. One page: write a line, see recent entries.

## Run

Live (your log):

```bash
cp .env.example .env
docker compose up --build
```

Open `http://localhost:3000`.

Test (other data, same code):

```bash
cp .env.test.example .env.test
docker compose --env-file .env.test up --build
```

Open `http://localhost:3001`. Both can run at once.

## Docs

- `SPEC.md` — v1 (page, table, two routes, compose)
- `TICKETS.md` — work queue
