#!/usr/bin/env bash

if [ "$FAUCET_URL" == "none" ] || [ "$FULLNODE_URL" == "none" ] || [ "$APTOS_NETWORK" == "none" ]; then
    echo "FAUCET_URL, FULLNODE_URL and APTOS_NETWORK must be set"
    exit 1
fi

cd /opt/antithesis-ts-test

pnpm run transfer_coin