# syntax=docker/dockerfile:1.3-labs
FROM rust:1.78 AS chef
RUN cargo install cargo-chef --version "0.1.66"
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends lld
ENV RUSTFLAGS="-C link-args=-fuse-ld=lld" CARGO_INCREMENTAL=0
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin pii-masker

# We do not need the Rust toolchain to run the binary!
FROM ubuntu:jammy AS runtime
WORKDIR app
RUN apt-get update \
 && apt-get install -y ca-certificates curl
COPY --from=builder /app/target/release/pii-masker /usr/local/bin
ENTRYPOINT ["/usr/local/bin/pii-masker"]
