# syntax=docker/dockerfile:1
FROM rust:alpine AS builder

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src src/
COPY resource resource/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release && \
    strip target/release/ip_pool

FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata

COPY --from=builder --chown=nobody:nobody \
    /app/target/release/ip_pool /usr/local/bin/ip_pool

USER nobody

EXPOSE 8080

CMD ["ip_pool"]
