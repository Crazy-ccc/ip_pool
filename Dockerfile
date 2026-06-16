# syntax=docker/dockerfile:1
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src src/
COPY resource resource/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release && \
    strip target/release/ip_pool

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder --chown=nobody:nogroup \
    /app/target/release/ip_pool /usr/local/bin/ip_pool

USER nobody

EXPOSE 8080

CMD ["ip_pool"]
