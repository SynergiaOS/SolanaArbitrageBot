#!/usr/bin/env bash
set -euo pipefail

# Solana Arbitrage Bot - Monitor-Only Runner
# Usage:
#   NETWORK=devnet ./scripts/start_monitor_only.sh
#   NETWORK=mainnet ./scripts/start_monitor_only.sh
#   DRY_RUN=1 NETWORK=devnet ./scripts/start_monitor_only.sh
#   CONFIG=./my-config.yaml NETWORK=devnet ./scripts/start_monitor_only.sh
#
# Env:
#   NETWORK:   devnet | mainnet | testnet (default: devnet)
#   DRY_RUN:   any non-empty value enables --dry-run
#   CONFIG:    path to YAML config (auto-detected if not provided)
#   RUST_LOG:  default info

NETWORK="${NETWORK:-devnet}"
DRY_RUN="${DRY_RUN:-}"
CONFIG="${CONFIG:-}"
RUST_LOG="${RUST_LOG:-info}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "[INFO] Starting monitor-only for NETWORK=${NETWORK}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[ERROR] cargo is required. Install Rust toolchain first."
  exit 2
fi

# Auto-select config if not provided
if [[ -z "${CONFIG}" ]]; then
  case "${NETWORK}" in
    devnet)
      if [[ -f "${ROOT_DIR}/config_devnet.yaml" ]]; then
        CONFIG="${ROOT_DIR}/config_devnet.yaml"
      else
        CONFIG="${ROOT_DIR}/config.yaml"
      fi
      ;;
    testnet)
      # Fallback to generic config if dedicated testnet config not present
      if [[ -f "${ROOT_DIR}/config.yaml" ]]; then
        CONFIG="${ROOT_DIR}/config.yaml"
      else
        CONFIG="${ROOT_DIR}/config_devnet.yaml"
      fi
      ;;
    mainnet)
      if [[ -f "${ROOT_DIR}/config.production.yaml" ]]; then
        CONFIG="${ROOT_DIR}/config.production.yaml"
      elif [[ -f "${ROOT_DIR}/config.production.enhanced.yaml" ]]; then
        CONFIG="${ROOT_DIR}/config.production.enhanced.yaml"
      else
        CONFIG="${ROOT_DIR}/config.yaml"
      fi
      ;;
    *)
      CONFIG="${ROOT_DIR}/config.yaml"
      ;;
  esac
fi

if [[ ! -f "${CONFIG}" ]]; then
  echo "[ERROR] Config file not found: ${CONFIG}"
  exit 3
fi

echo "[INFO] Using config: ${CONFIG}"
echo "[INFO] Building (monitor-only)..."
cargo build --release --no-default-features --features monitor

BIN="${ROOT_DIR}/target/release/solana-arbitrage-bot"
if [[ ! -x "${BIN}" ]]; then
  echo "[ERROR] Binary not found: ${BIN}"
  exit 4
fi

ARGS=(--network "${NETWORK}" --config "${CONFIG}")
if [[ -n "${DRY_RUN}" ]]; then
  ARGS+=(--dry-run)
fi

export RUST_LOG="${RUST_LOG}"
echo "[INFO] RUST_LOG=${RUST_LOG}"
echo "[INFO] Executing: ${BIN} ${ARGS[*]}"
exec "${BIN}" "${ARGS[@]}"