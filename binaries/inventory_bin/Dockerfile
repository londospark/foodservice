FROM rust:1.89-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY binaries ./binaries
COPY xtask ./xtask

RUN cargo build --release -p inventory_bin

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/inventory_bin /usr/local/bin/inventory_bin

EXPOSE 3001

CMD ["inventory_bin"]
