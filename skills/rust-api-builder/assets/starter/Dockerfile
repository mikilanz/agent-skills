FROM rust:1.90-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# cache dependencies: build once with a stub main, then with the real sources
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && touch src/lib.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
# embed_migrations! reads this folder at compile time
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs && cargo build --release


FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/rust-api-starter /usr/local/bin/rust-api-starter
COPY public ./public
RUN mkdir -p public/assets/files

ENV APP_PORT=8000
EXPOSE 8000

CMD ["rust-api-starter"]
