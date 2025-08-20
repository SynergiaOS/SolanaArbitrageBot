# 🚀 Solana Arbitrage Bot - Multi-stage Docker Build
# Optimized for security, performance, and minimal size

# Build stage
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libudev-dev \
    libusb-1.0-0-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency files first (for better caching)
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src

# Build the actual application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libudev1 \
    libusb-1.0-0 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN groupadd -r arbitrage && useradd -r -g arbitrage arbitrage

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/solana-arbitrage-bot /usr/local/bin/solana-arbitrage-bot

# Copy configuration files
COPY config.yaml ./config.example.yaml
COPY setup_ledger_ubuntu.sh ./

# Create directories for data
RUN mkdir -p /app/data /app/logs && \
    chown -R arbitrage:arbitrage /app

# Switch to non-root user
USER arbitrage

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep solana-arbitrage-bot || exit 1

# Expose metrics port (if implemented)
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Default command
CMD ["solana-arbitrage-bot", "--config", "config.yaml"]

# Labels for metadata
LABEL org.opencontainers.image.title="Solana Arbitrage Bot"
LABEL org.opencontainers.image.description="Advanced Solana arbitrage bot with Ledger integration"
LABEL org.opencontainers.image.vendor="SynergiaOS"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.source="https://github.com/SynergiaOS/SolanaArbitrageBot"
