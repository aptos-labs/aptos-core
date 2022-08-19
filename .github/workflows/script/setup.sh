#!/bin/bash

#git checkout rosetta-stable

# Build with the rosetta-stable branch (I use the git hash here)
# export GIT_REF=dd2bfaf31fcd31b79b9f65e7fbc812338256bbd0

# nohup docker/rosetta/docker-build-rosetta.sh  > /dev/null 2>&1 &

# # Build data path for node & files
# mkdir -p data

# # Download node config (and fix the paths)
# curl -s https://raw.githubusercontent.com/aptos-labs/aptos-core/rosetta-stable/config/src/config/test_data/public_full_node.yaml 2> /dev/null | sed 's/\.\//\/opt\/aptos\//g' > data/fullnode.yaml

# # Download waypoint and genesis necessary to start a node
# curl -s -o data/genesis.blob https://rosetta.aptosdev.com/genesis.blob
# curl -s -o data/waypoint.txt https://rosetta.aptosdev.com/waypoint.txt

# sleep 3000

# # Run the node in online remote mode (detached mode)
# nohup docker run -d -p 8082:8082 --rm -v $(pwd)/data:/opt/aptos aptos-core:rosetta-$GIT_REF online-remote --rest-api-url https://rosetta.aptosdev.com > /dev/null 2>&1 &

# sleep 60

# block_tip=($(curl -s --location --request POST 'http://localhost:8082/network/status' \
# --header 'Content-Type: application/json' \
# --data-raw '{
#     "network_identifier": {
#             "blockchain": "aptos",
#             "network": "TESTING"
#         }
# }' | python3 -c 'import json,sys;obj=json.load(sys.stdin);print(obj["current_block_identifier"]["index"])'))

echo $(seq $START_INDEX $BLKS_PER_CHUNK 5000 | jq -cnR '[inputs | select(length>0)]')
