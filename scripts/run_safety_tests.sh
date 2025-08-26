#!/usr/bin/env bash
set -euo pipefail

# Run SafetyChecker-related tests and config loader test in sniper bin
export RUST_LOG=info
cargo test sniper::safety -- --nocapture
cargo test --bin sniper tests::maps_min_liquidity_from_safety_yaml -- --nocapture
