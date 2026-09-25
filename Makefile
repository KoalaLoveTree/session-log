# Commands for this repo.
.DEFAULT_GOAL := help
.PHONY: help build-live build-test db-test test cov

help:
	@printf '%s\n' \
		'make build-live   live site on :3000' \
		'make build-test   test site on :3001' \
		'make db-test      test database on :5433' \
		'make test         API tests' \
		'make cov          API tests and the coverage report'

build-live:
	test -f .env || cp .env.example .env
	docker compose up --build -d

build-test:
	test -f .env.test || cp .env.test.example .env.test
	docker compose --env-file .env.test up --build -d

db-test:
	test -f .env.test || cp .env.test.example .env.test
	docker compose --env-file .env.test up -d --wait db

test: db-test
	set -a && . ./.env.test && set +a && \
	cd api && \
	DATABASE_URL="postgres://$${POSTGRES_USER}:$${POSTGRES_PASSWORD}@127.0.0.1:$${POSTGRES_HOST_PORT}/$${POSTGRES_DB}" \
	cargo test

cov: db-test
	set -a && . ./.env.test && set +a && \
	cd api && \
	DATABASE_URL="postgres://$${POSTGRES_USER}:$${POSTGRES_PASSWORD}@127.0.0.1:$${POSTGRES_HOST_PORT}/$${POSTGRES_DB}" \
	cargo llvm-cov --html
	@printf 'file://%s/api/target/llvm-cov/html/index.html\n' "$$(pwd)"
