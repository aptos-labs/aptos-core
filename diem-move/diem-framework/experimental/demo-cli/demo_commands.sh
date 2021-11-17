# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

# local setup
export ROOT_KEY="local_root.key"
export TC_KEY="local_root.key"
export BARS_ADDRESS="0x06505CCD81E562B524D8F656ABD92A15"
export JSON_RPC_ENDPOINT="http://0.0.0.0:8080"
export REST_API_ENDPOINT="localhost:8080"
# run a local node
cargo run --bin diem-node -- --lazy --test --open-publishing --genesis-modules diem-move/diem-framework/experimental/releases/artifacts/current --seed 0000000000000000000000000000000000000000000000000000000000000000

cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path $TC_KEY --account-address 0xB1E55ED create-basic-account $BARS_ADDRESS 8bf3d7c0b381385c06bdfe37e1230cc8
cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path bars_account.key --account-address $BARS_ADDRESS create-basic-account 0x3132E2B5216A46DFCF8154079954C129 45ecaf56addd68aa8be08e2763c09857
cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path bars_account.key --account-address $BARS_ADDRESS create-basic-account 0x1A08E8165BB9225702495E8CB6E57E61 5839ada2d75b6c43f194123337a11c5e

# get all the resource types of the newly created account
curl ${REST_API_ENDPOINT}/accounts/${BARS_ADDRESS}/resources | jq '.[] | .type'
# look at the event
curl "${REST_API_ENDPOINT}/accounts/0xB1E55ED/transactions?start=0&limit=1" | jq

# initialize NFT
cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path $ROOT_KEY --account-address 0xA550C18 init-multi-token
# verify resources
curl ${REST_API_ENDPOINT}/accounts/0xA550C18/resources | jq '.[] | select(.type | contains("NFT"))'

# register user (give BARS capability to mint on behalf of the user)
cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path user_account1.key --account-address 0x3132E2B5216A46DFCF8154079954C129 register-bars-user
# Look at all the resources added
curl ${REST_API_ENDPOINT}/accounts/0x3132E2B5216A46DFCF8154079954C129/resources | jq '.[] | .type'
# Creation delegation resource
curl ${REST_API_ENDPOINT}/accounts/0x3132E2B5216A46DFCF8154079954C129/resources | jq '.[] | select(.type | contains("CreationDelegation"))'

# register another user
cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path user_account2.key --account-address 0x1A08E8165BB9225702495E8CB6E57E61 register-bars-user
cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path bars_account.key --account-address $BARS_ADDRESS mint-bars-nft --creator-addr 0x3132E2B5216A46DFCF8154079954C129 --creator-name "Some Name" --content-uri "www.diem.com" --amount 100
# Token data collection
curl ${REST_API_ENDPOINT}/accounts/0x3132E2B5216A46DFCF8154079954C129/resources | jq '.[] | select(.type | contains("TokenDataCollection"))'
# Token gallery
curl ${REST_API_ENDPOINT}/accounts/0x3132E2B5216A46DFCF8154079954C129/resources | jq '.[] | select(.type | contains("NFTGallery"))'
# Transfer NFT
cargo run -- --jsonrpc-endpoint $JSON_RPC_ENDPOINT --account-key-path user_account1.key --account-address 0x3132E2B5216A46DFCF8154079954C129 transfer-bars-nft --to 0x1A08E8165BB9225702495E8CB6E57E61 --amount 10 --creator 0x3132E2B5216A46DFCF8154079954C129 --creation-num 2
# Verify the new balance of the second user
curl ${REST_API_ENDPOINT}/accounts/0x1A08E8165BB9225702495E8CB6E57E61/resources | jq '.[] | select(.type | contains("NFTGallery"))'
