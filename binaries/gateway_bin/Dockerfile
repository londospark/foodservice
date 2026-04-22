FROM rust:1.89-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY binaries ./binaries
COPY xtask ./xtask

RUN cargo build --release -p gateway_bin

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gateway_bin /usr/local/bin/gateway_bin

EXPOSE 3000

CMD ["gateway_bin"]
