#!/usr/bin/env bash
set -euo pipefail

# Solana Arbitrage Bot - Monitor-Only Sanity Check
# Uruchamia bota (DRY_RUN) na devnet / mainnet / testnet max 60s
# i weryfikuje, że wystartowała pętla arbitrażu.
#
# Użycie:
#   NETWORK=devnet ./scripts/sanity_check_monitor.sh
#   NETWORK=mainnet ./scripts/sanity_check_monitor.sh
# Env:
#   NETWORK:   devnet | mainnet | testnet (default: devnet)
#   CONFIG:    opcjonalna ścieżka do YAML
#   TIMEOUT:   czas w sekundach (default: 60)
#   RUST_LOG:  domyślnie info

NETWORK="${NETWORK:-devnet}"
CONFIG="${CONFIG:-}"
TIMEOUT="${TIMEOUT:-60}"
RUST_LOG="${RUST_LOG:-info}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[ERROR] cargo is required. Install Rust toolchain first."
  exit 2
fi

# Dobór configu
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

# Build monitor-only jeśli brak binarki
BIN="${ROOT_DIR}/target/release/solana-arbitrage-bot"
if [[ ! -x "${BIN}" ]]; then
  echo "[INFO] Building (monitor-only)..."
  cargo build --release --no-default-features --features monitor
fi

LOG="/tmp/sab_sanity_$$.log"
echo "[INFO] Running sanity-check (timeout=${TIMEOUT}s), log: ${LOG}"
set +e
RUST_LOG="${RUST_LOG}" timeout "${TIMEOUT}" "${BIN}" --network "${NETWORK}" --config "${CONFIG}" --dry-run > "${LOG}" 2>&1
RC=$?
set -e

# Weryfikacje
if grep -q "Starting arbitrage loop" "${LOG}"; then
  echo "[OK] Pętla arbitrażu wystartowała."
else
  echo "[FAIL] Nie znaleziono startu pętli arbitrażu."
  tail -n 50 "${LOG}" || true
  exit 1
fi

if grep -q "Waiting for price data" "${LOG}"; then
  echo "[OK] Oczekiwanie na dane cenowe wykryte (monitor pracuje)."
fi

echo "[INFO] Ostatnie 20 linii logu:"
tail -n 20 "${LOG}" || true

echo "[SUCCESS] Sanity-check zakończony powodzeniem."