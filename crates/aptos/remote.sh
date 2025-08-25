#!/bin/bash

set -e

rm -rf sess

# initialize session
cargo run -p aptos -- move sim init --path sess --network devnet --api-key aptoslabs_6foGZ5JtuHL_GMdiektM75QvVVVUGxt5wzxKZMPsgxbxb

# fund account with 1 APT
cargo run -p aptos -- move sim fund --session sess --account default --amount 100000000

# transfer 100 Octa to self
cargo run -p aptos -- move run --session sess --function-id 0x1::aptos_account::transfer --args address:default u64:100
