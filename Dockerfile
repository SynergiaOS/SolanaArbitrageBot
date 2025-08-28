# Multi-stage Dockerfile for production

# 1) Builder stage
FROM rust:1.79 as builder
ARG CARGO_FEATURES=monitor
WORKDIR /app
# Cache dependencies
COPY Cargo.toml Cargo.lock ./
# Create fake sources to warm cache
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && \
    mkdir -p src/bin && echo "fn main(){}" > src/bin/sniper.rs && \
    echo "fn main(){}" > demo_discord.rs
RUN cargo build --release --no-default-features --features $CARGO_FEATURES || true
# Now copy real sources
COPY . .
RUN cargo build --release --no-default-features --features $CARGO_FEATURES

# 2) Runtime stage (debian slim)
FROM debian:stable-slim as runtime
# Install minimal runtime deps if needed (ca-certificates for HTTPS)
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Create non-root user and group
RUN groupadd -r arbitrage && useradd -r -g arbitrage arbitrage

# Create app directories and set permissions
RUN mkdir -p /app/data /app/logs && chown -R arbitrage:arbitrage /app

# Copy binary (root will write, but ownership already set for /app)
COPY --from=builder /app/target/release/solana-arbitrage-bot /usr/local/bin/solana-arbitrage-bot
# Optional: include other binaries if used in production
# COPY --from=builder /app/target/release/gepa_optimizer /usr/local/bin/gepa_optimizer

# Default config path inside container
COPY config.production.yaml /app/config.production.yaml
RUN chown arbitrage:arbitrage /app/config.production.yaml

# Switch to non-root user
USER arbitrage

ENV RUST_LOG=info
EXPOSE 3001
CMD ["/usr/local/bin/solana-arbitrage-bot", "--config", "/app/config.production.yaml"]

