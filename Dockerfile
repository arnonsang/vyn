# syntax=docker/dockerfile:1

FROM rust:1.91-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY proto ./proto

RUN cargo build --release --bin vyn

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates netcat-openbsd openssh-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/vyn /usr/local/bin/vyn

RUN mkdir -p /data && useradd -u 10001 -m vyn && chown vyn:vyn /data
USER vyn

EXPOSE 50051

ENTRYPOINT ["vyn", "serve", "--relay"]
CMD ["--port", "50051", "--data-dir", "/data"]
