# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Repository Overview
- **Language**: Rust (async with Tokio runtime)
- **Domain**: Solana DEX arbitrage bot targeting SOL/USDC pairs across Raydium and Orca
- **Architecture**: KISS principle - single process, ~3000 lines of code
- **Integration**: Jupiter aggregator for routing, optional Ledger hardware wallet support, Discord webhook notifications
- **Primary binary**: `solana-arbitrage-bot` (src/main.rs)
- **Performance targets**: <300ms latency, >$50/day profit, >95% uptime

## Key Modules
- `monitor.rs`: Real-time price monitoring via GeckoTerminal API and Jupiter quotes
- `calculator.rs`: Arbitrage opportunity detection with dynamic position sizing and confidence scoring
- `executor.rs`: Transaction execution via Jupiter API v6 with on-chain profit verification
- `safety.rs`: Risk management with position limits, daily loss tracking, and circuit breakers
- `ledger.rs`: Hardware wallet integration (partial implementation)
- `discord.rs`: Alert system for opportunities, profits, and errors
- `sniper/`: Alternative token sniping strategy (separate module)

## Essential Documentation
- `README.md`: Project goals, architecture overview, quick start guide
- `ADVANCED_STRATEGIES.md`: Market analysis, optimization strategies, performance tuning
- `SECURITY.md`: Security best practices, vulnerability reporting, incident response
- `DEVNET_TESTING.md`: Step-by-step testing guide before mainnet deployment
- `INFRASTRUCTURE_SECURITY.md`: Server setup and operational security

Important note about config files
- src expects a Config matching structs in src/lib.rs (rpc, wallet, dex, limits, execution, optional discord).
- README.md shows a config layout for arbitrage (rpc, wallet, dex.raydium/orca, limits, etc.).
- The committed config.yaml appears to target a separate “sniper” workflow (sniper:, safety:, risk:), which does not match the arbitrage Config schema. To run arbitrage, provide a config file that conforms to the structs in src/lib.rs (see “Config example for arbitrage” below) and pass it via --config.

Common commands
- Build fast (debug):
  cargo build
- Build optimized:
  cargo build --release
- Run (arbitrage bot):
  cargo run -- --config ./config.arb.yaml --dry-run --network mainnet
  cargo run --release -- --config ./config.arb.yaml --network mainnet
- CLI flags (from src/main.rs):
  --dry-run                    Run without submitting real transactions
  --network <mainnet|testnet|devnet>
  --config <path>             Path to YAML config
  --max_position <f64>        Override max position (SOL)
  --test_ledger               Test Ledger connection then exit
  --ledger_path <path>        Ledger derivation path (default m/44'/501'/0'/0')
  --test_apis                 Probe Solana RPC + Jupiter API then exit
- Logs:
  RUST_LOG=info cargo run -- ...
  RUST_LOG=debug cargo test -- --nocapture
- Unit tests:
  cargo test
  cargo test -- --nocapture
- Run a single unit test by name (pattern match):
  cargo test test_profitable_arbitrage -- --nocapture
- Integration test (tests/integration.rs):
  cargo test --test integration -- --nocapture
- Lint (if installed):
  cargo clippy --all-targets -- -D warnings
- Format:
  cargo fmt --all

Docker
- Build image:
  docker build -t solana-arbitrage-bot .
- Run (mount wallet and config; adjust paths):
  docker run --rm -it \
    -e RUST_LOG=info \
    -v "$(pwd)/wallet.json:/app/wallet.json:ro" \
    -v "$(pwd)/config.arb.yaml:/app/config.yaml:ro" \
    --name sol-arb solana-arbitrage-bot
- Notes:
  - The Dockerfile copies the built binary as /usr/local/bin/solana-arbitrage-bot and defaults to CMD ["solana-arbitrage-bot", "--config", "config.yaml"].
  - Ensure the mounted config matches the arbitrage schema (see below), not the sniper schema.

Optional binaries
- Additional Cargo bins defined:
  - demo-discord: cargo run --bin demo-discord
  - sniper: cargo run --bin sniper (file may be missing; verify src/bin/sniper.rs exists before using)

Config example for arbitrage
Create config.arb.yaml in the repo (do not commit secrets) that aligns with src/lib.rs types:

rpc:
  url: "https://api.mainnet-beta.solana.com"
  ws_url: "wss://api.mainnet-beta.solana.com"
wallet:
  path: "./wallet.json"
dex:
  raydium:
    program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    sol_usdc_pool: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"
  orca:
    program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
    sol_usdc_pool: "HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"
limits:
  max_position_sol: 10.0
  min_profit_percent: 0.3
  min_profit_usd: 1.0
  max_slippage_percent: 0.5
  max_daily_loss_usd: 100.0
  max_daily_trades: 30
execution:
  priority_fee_lamports: 10000
  simulation_required: true
  max_retries: 3
discord:
  enabled: false
  webhook_url: "{{DISCORD_WEBHOOK_URL}}"

High-level architecture and data flow
- Main loop (src/main.rs):
  - Parses CLI, loads Config, sets up shared state (Arc<Mutex<...>>), channels, and components.
  - Spawns monitor tasks for Raydium and Orca to continuously refresh prices and push PriceUpdate messages.
  - A price handler task periodically emits Discord price updates (rate-limited) if enabled.
  - The arbitrage loop reads latest prices, computes opportunities with ProfitCalculator, gates with SafetyGuard, executes via TransactionExecutor, then on-chain verifies profit by parsing token balances (USDC) and updates daily stats; Discord alerts for startup, opportunities, profit, and errors.
- monitor.rs:
  - DexMonitor::new wires shared state and pool addresses from config.
  - Currently polls GeckoTerminal pool endpoints for SOL/USDC on each DEX as a pragmatic source; includes helper to query Jupiter quote API.
  - Emits PriceUpdate over an mpsc channel and writes to shared state.
- calculator.rs:
  - ProfitCalculator::calculate_opportunity determines direction (buy cheaper DEX, sell higher), sizes position dynamically based on spread and optional market conditions, and computes net profit after gas, fees, slippage, and price impact, returning an ArbitrageOpportunity with a confidence score.
- safety.rs:
  - SafetyGuard enforces max position, daily loss and trade limits, tracks trade history and recent losses, provides pre_trade_check and reset_daily_limits. It can trigger emergency_stop for critical conditions.
- executor.rs:
  - TransactionExecutor loads wallet (keypair JSON; Ledger path present but not fully implemented), sets RPC client, and executes trades.
  - For live mode, obtains a Jupiter quote (v6), requests a swap transaction from the swap endpoint, decodes bincode tx, signs with the keypair, optionally simulates, then sends and confirms. verify_transaction fetches the confirmed tx and computes realized profit from USDC token balance delta.
  - Provides a placeholder execute_direct_swap path for future DEX-specific routing.
- lib.rs:
  - Exposes modules, shared types, and the Config schema (rpc, wallet, dex, limits, execution, discord). SharedState holds latest prices and daily stats for orchestration.

Notes and caveats for agents
- Config schema mismatch: Ensure the config you pass to --config matches src/lib.rs (arbitrage), not the shipped sniper-focused config.yaml. Prefer creating a separate config.arb.yaml.
- Secrets: Do not commit real wallet.json or Discord webhook URLs. When running, mount or point to local files; for commands, reference placeholders like {{DISCORD_WEBHOOK_URL}} and {{WALLET_JSON}}.
- Network I/O: Some tests and runtime features call external APIs (Solana RPC, Jupiter, GeckoTerminal). Expect network variability; integration tests tolerate failures and print warnings.
- Ledger: executor.rs and ledger.rs include scaffolding; signing via Ledger is not fully implemented. Use keypair JSON for now or run with --dry-run.
- Additional bin targets defined in Cargo.toml may require files that aren’t present (sniper). Verify before invoking.

Pulled rules from other tooling
- .github/copilot-instructions.md:
  - Mentions custom tools named byterover-retrieve-knowledge and byterover-store-knowledge. If those tools are available in your environment, retrieve context before tasks and store critical information after successful tasks. Otherwise, ignore as non-applicable.

Quick validation workflows
- Sanity check external connectivity without trading:
  cargo run -- --test_apis
- Dry run with live prices (no tx):
  RUST_LOG=info cargo run -- --config ./config.arb.yaml --dry-run --network mainnet
- Full test suite (unit + integration):
  cargo test --all -- --nocapture

Alignment with README.md
- The README emphasizes a KISS architecture, arbitrage across Raydium/Orca SOL/USDC, and target performance metrics. The commands above reflect the README’s suggested flows (dry run, testnet/mainnet flags), adapted to the current CLI and Config types in src.

