# Stage 1: Builder
FROM rust:1.77-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/carbide-core/Cargo.toml crates/carbide-core/Cargo.toml
COPY crates/carbide-provider/Cargo.toml crates/carbide-provider/Cargo.toml
COPY crates/carbide-discovery/Cargo.toml crates/carbide-discovery/Cargo.toml
COPY crates/carbide-client/Cargo.toml crates/carbide-client/Cargo.toml
COPY crates/carbide-reputation/Cargo.toml crates/carbide-reputation/Cargo.toml
COPY crates/carbide-crypto/Cargo.toml crates/carbide-crypto/Cargo.toml

# Create empty source files for dependency caching
RUN mkdir -p crates/carbide-core/src && echo "" > crates/carbide-core/src/lib.rs && \
    mkdir -p crates/carbide-provider/src && echo "fn main() {}" > crates/carbide-provider/src/main.rs && \
    mkdir -p crates/carbide-discovery/src && echo "fn main() {}" > crates/carbide-discovery/src/main.rs && \
    mkdir -p crates/carbide-client/src && echo "fn main() {}" > crates/carbide-client/src/main.rs && \
    mkdir -p crates/carbide-reputation/src && echo "" > crates/carbide-reputation/src/lib.rs && \
    mkdir -p crates/carbide-crypto/src && echo "" > crates/carbide-crypto/src/lib.rs

# Build dependencies only (cached layer)
RUN cargo build --release --bin carbide-provider 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Touch source files to invalidate cache and rebuild with real source
RUN touch crates/*/src/*.rs

# Build the actual binary
RUN cargo build --release --bin carbide-provider

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r carbide && useradd -r -g carbide -m carbide

# Create data and config directories
RUN mkdir -p /data /config && chown -R carbide:carbide /data /config

COPY --from=builder /app/target/release/carbide-provider /usr/local/bin/carbide-provider

USER carbide

VOLUME ["/data", "/config"]
EXPOSE 8080

ENTRYPOINT ["carbide-provider"]
