#!/bin/bash

BIN="cargo run -- --identity encrypted_ec_key.pem --ethereum-url $ETH_URL --arbitrum-url $ARB_URL --base-url $BAS_URL --forward-chains=all"
LOG_DIR="./logs"
DELAY=10         

mkdir -p "$LOG_DIR"

while true; do
    TIMESTAMP=$(date +"%Y-%m-%d_%H-%M-%S")
    LOG_FILE="$LOG_DIR/relayer_$TIMESTAMP.log"

    echo "[$TIMESTAMP] Starting relayer..."
    $BIN > "$LOG_FILE" 2>&1

    EXIT_CODE=$?
    echo "[$(date +"%Y-%m-%d_%H-%M-%S")] Relayer exited with code $EXIT_CODE. Restarting in $DELAY seconds..."
    sleep $DELAY
done
