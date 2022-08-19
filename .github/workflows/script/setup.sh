#!/bin/bash

# block_tip=($(curl -s --location --request POST 'http://localhost:8082/network/status' \
# --header 'Content-Type: application/json' \
# --data-raw '{
#     "network_identifier": {
#             "blockchain": "aptos",
#             "network": "TESTING"
#         }
# }' | python3 -c 'import json,sys;obj=json.load(sys.stdin);print(obj["current_block_identifier"]["index"])'))

echo $(seq $START_INDEX $BLKS_PER_CHUNK 13100000 | jq -cnR '[inputs | select(length>0)]')
