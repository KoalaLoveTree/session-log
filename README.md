# session log

Personal session log. One page: write a line, see recent entries.

```bash
git clone https://github.com/KoalaLoveTree/session-log.git
cd session-log
```

![The log: a text box and recent entries](screenshot.png)

## Run

Live (your log):

```bash
cp .env.example .env
docker compose up --build
```

Open `http://localhost:3000`. On a phone on the same Wi‑Fi: `http://<lan-ip>:3000` (on the PC, `hostname -I`).

Test (other data, same code):

```bash
cp .env.test.example .env.test
docker compose --env-file .env.test up --build
```

Open `http://localhost:3001`. Both can run at once.

## Start on boot (Linux)

Optional. After live compose works, a user systemd unit can start it at boot.

Write `~/.config/systemd/user/session-log.service`. Set `WorkingDirectory` to this clone:

```
[Unit]
Description=session-log live compose stack

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/path/to/session-log
ExecStart=/usr/bin/docker compose up -d
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user enable --now session-log.service
loginctl enable-linger
```

Your user needs to be able to run `docker` (often the `docker` group). Docker itself should start at boot (`systemctl enable docker`).

Linger starts **your** user services at boot, even at the login screen, before anyone unlocks the session.

## Rust

```bash
cd api
cargo fmt
cargo clippy -- -D warnings
DATABASE_URL=postgres://lr:lr@127.0.0.1:5433/lr cargo test
```

`cargo test` needs the test stack (`docker compose --env-file .env.test up --build`) so Postgres is on `127.0.0.1:5433`. Tests create their own databases; they do not write your log.

## Docs

- `SPEC.md` — v1 (page, table, two routes, compose)
- `TICKETS.md` — work queue
- [http://localhost:3000/api/docs](http://localhost:3000/api/docs) — API (Swagger)

GitHub Actions (`ci`) runs fmt, clippy, `cargo test`, and `cargo build` on push.
