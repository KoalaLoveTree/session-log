FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY swagger-dark.css ./swagger-dark.css
COPY migrations ./migrations
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=build /app/target/release/api /usr/local/bin/api
CMD ["api"]
