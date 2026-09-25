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
```

```bash
make build-live
make build-test
make test
make cov
```

`make build-live` builds and starts the live site on port 3000. `make build-test` does the same for the test site on port 3001. Both stay running in the background.

`make test` starts the test database when it is down, then runs `cargo test` in `api/`. Postgres listens on `127.0.0.1:5433`. Tests create their own databases; they do not write your log.

`make cov` runs those same tests under `cargo llvm-cov` and writes `api/target/llvm-cov/html/index.html`. It then prints a `file://` link; click that to open the report. The site on port 3000 does not serve it. Coverage needs the `llvm-tools-preview` rustup component and `cargo-llvm-cov` (`cargo install cargo-llvm-cov --locked`).

## Docs

- `SPEC.md` — how the modules meet (`api/SPEC.md` routes, `web/SPEC.md` screen)
- `TICKETS.md` — work queue
- [http://localhost:3000/api/docs](http://localhost:3000/api/docs) — API (Swagger)

GitHub Actions (`ci`) runs fmt, clippy, the tests under `cargo llvm-cov`, and `cargo build` on push. The job log prints the coverage summary. A failing test fails the job.
