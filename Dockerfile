# Multi-stage build for Traveling Cook Calculator API

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM rust:1.75-slim as builder

WORKDIR /build

# Install dependencies
RUN apt-get update && apt-get install -y \
    postgresql-client \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Install diesel-cli
RUN cargo install diesel_cli --no-default-features --features postgres --locked

# Copy source code
COPY . .

# Build application
RUN cargo build --release

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    postgresql-client \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/tcc_server /app/tcc_server

# Copy migrations
COPY migrations /app/migrations

# Set environment variables
ENV RUST_LOG=info
ENV ADDR=0.0.0.0:3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Expose port
EXPOSE 3000

# Run application
CMD ["/app/tcc_server"]
