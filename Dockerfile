# syntax=docker/dockerfile:1.4
# Fast optimized build for Polarway gRPC server

FROM rustlang/rust:nightly-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_BUILD_JOBS=8 \
    CARGO_INCREMENTAL=0 \
    RUSTFLAGS="-C codegen-units=16 -C opt-level=3"

# Install build dependencies
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev protobuf-compiler libprotobuf-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace Cargo.toml and exclude problematic members
COPY Cargo.toml /build/
RUN sed -i '/docs\/source\/src\/rust/d' /build/Cargo.toml && \
    sed -i '/py-polars\/runtime/d' /build/Cargo.toml && \
    sed -i '/pyo3-polars/d' /build/Cargo.toml

# Copy necessary crates and polarway-grpc
COPY crates/ /build/crates/
COPY polarway-grpc/ /build/polarway-grpc/
COPY polarway-sources/ /build/polarway-sources/
COPY polarway-distributed/ /build/polarway-distributed/
COPY polarway-python/ /build/polarway-python/
COPY proto/ /build/proto/

# Build in release mode with optimizations
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cd polarway-grpc && \
    cargo build --release --target-dir /build/target && \
    cp /build/target/release/polarway-grpc /polarway-grpc

# Runtime stage - must match builder's GLIBC version
FROM debian:trixie-slim

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    libssl3 ca-certificates netcat-openbsd && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /polarway-grpc /app/polarway-grpc

# Default bind to all interfaces
ENV POLARWAY_BIND_ADDRESS=0.0.0.0:50052

EXPOSE 50052

# Simple health check using nc
RUN apt-get update && apt-get install -y --no-install-recommends netcat-openbsd && rm -rf /var/lib/apt/lists/*

CMD ["/app/polarway-grpc"]
