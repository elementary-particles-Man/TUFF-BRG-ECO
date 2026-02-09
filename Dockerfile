FROM rust:latest AS builder
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY tuff-brg ./tuff-brg
COPY tuff-db ./tuff-db

RUN cargo build --release -p tuff_brg

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/tuff_brg /usr/local/bin/tuff_brg

EXPOSE 8787
VOLUME ["/app/_tuffdb"]

ENV TUFF_HISTORY_OUT=history_out
ENTRYPOINT ["/usr/local/bin/tuff_brg"]
