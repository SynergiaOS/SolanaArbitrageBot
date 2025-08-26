#!/usr/bin/env bash
set -euo pipefail

# Setup devnet wallet and funding
CONFIG_FILE=${1:-config_devnet.yaml}
WALLET_FILE=${2:-devnet_wallet.json}

solana config set --url https://api.devnet.solana.com

if [ ! -f "$WALLET_FILE" ]; then
  echo "Generating devnet wallet: $WALLET_FILE"
  solana-keygen new --no-bip39-passphrase --outfile "$WALLET_FILE"
fi

PUBKEY=$(solana-keygen pubkey "$WALLET_FILE")
echo "Devnet wallet pubkey: $PUBKEY"

# Airdrop a few times to ensure sufficient SOL
for i in 1 2 3; do
  solana airdrop 2 "$PUBKEY" || true
  sleep 2
done

solana balance "$PUBKEY"

echo "Ensure $CONFIG_FILE has rpc/ws set to devnet and wallet path: $WALLET_FILE"

