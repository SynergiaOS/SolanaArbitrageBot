# Project Overview

This project is a Solana arbitrage bot written in Rust. Its main purpose is to identify and execute arbitrage opportunities between the Raydium and Orca DEXes on the SOL/USDC pair. The bot monitors prices on both exchanges in real-time, calculates potential profits after fees and slippage, and executes trades using the Jupiter aggregator for optimal routing.

The project is well-structured and includes features like:
- Real-time price monitoring using the GeckoTerminal API.
- A sophisticated profit calculator that considers gas fees, slippage, and DEX fees.
- Trade execution via the Jupiter aggregator for best price execution.
- A safety module to prevent losses with features like daily loss limits, trade limits, and position size limits.
- Discord integration for real-time notifications.
- Command-line interface for configuration and control.

# Building and Running

## Building the project

To build the project, use the following command:

```bash
cargo build --release
```

## Running the bot

The bot can be run with various command-line arguments to control its behavior.

### Dry run (no real transactions)

```bash
./target/release/solana-arbitrage-bot --dry-run
```

### Testnet

```bash
./target/release/solana-arbitrage-bot --network testnet
```

### Mainnet (use with caution)

```bash
./target/release/solana-arbitrage-bot --network mainnet --max-position 10
```

## Testing

The project includes unit tests that can be run with:

```bash
cargo test
```

# Development Conventions

- The project follows standard Rust conventions and is well-documented with inline comments.
- The code is organized into modules with clear responsibilities:
    - `main.rs`: Entry point and main application logic.
    - `monitor.rs`: Price monitoring for DEXes.
    - `calculator.rs`: Profit calculation for arbitrage opportunities.
    - `executor.rs`: Trade execution.
    - `safety.rs`: Safety checks and limits.
    - `discord.rs`: Discord notifications.
- The project uses a `config.yaml` file for configuration, which is loaded at startup.
- The `clap` crate is used for command-line argument parsing.
- The `anyhow` and `thiserror` crates are used for error handling.
- Asynchronous operations are handled using the `tokio` runtime.
