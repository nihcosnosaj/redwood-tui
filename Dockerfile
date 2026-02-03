# Stage 1: Build
FROM rust:1.75-slim-bookworm AS builder

# Install system dependencies for SQLite and OpenSSL
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the source code
COPY . .

# Build the release binary
RUN cargo build --release

# Stage 2: Runtime 
FROM debian:bookworm-slim

# Install runtime dependencies (SQLite and SSL certificates for the API)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from the builder
COPY --from=builder /app/target/release/redwood-tui .

# Ensure the data directory exists for the CSV/DB
RUN mkdir -p data

# Run the app
ENTRYPOINT ["./redwood-tui"]