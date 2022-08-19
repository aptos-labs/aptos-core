#!/bin/bash

END_INDEX=$(($START_INDEX+$COUNT))
echo "index ranges", $START_INDEX, $END_INDEX

# git checkout rosetta-stable

# # Build with the rosetta-stable branch (I use the git hash here)
# export GIT_REF=dd2bfaf31fcd31b79b9f65e7fbc812338256bbd0

# docker/rosetta/docker-build-rosetta.sh

# # Build data path for node & files
# mkdir -p data

# # Download node config (and fix the paths)
# curl https://raw.githubusercontent.com/aptos-labs/aptos-core/rosetta-stable/config/src/config/test_data/public_full_node.yaml 2> /dev/null | sed 's/\.\//\/opt\/aptos\//g' > data/fullnode.yaml

# # Download waypoint and genesis necessary to start a node
# curl -o data/genesis.blob https://rosetta.aptosdev.com/genesis.blob
# curl -o data/waypoint.txt https://rosetta.aptosdev.com/waypoint.txt

# # Run the node in online remote mode (detached mode)
# docker run -d -p 8082:8082 --rm -v $(pwd)/data:/opt/aptos aptos-core:rosetta-$GIT_REF online-remote --rest-api-url https://rosetta.aptosdev.com


# # downloading cli
# curl -sSfL https://raw.githubusercontent.com/coinbase/rosetta-cli/master/scripts/install.sh | sh -s

# sleep 1500

# echo "start check:data"
# ./bin/rosetta-cli --configuration-file crates/aptos-rosetta/rosetta_cli.json check:data --start-block 100 --end-block 500
