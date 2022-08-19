#!/bin/bash

END_INDEX=$(($START_INDEX+$COUNT))
echo "index ranges", $START_INDEX, $END_INDEX

# git checkout rosetta-stable

# Build with the rosetta-stable branch (I use the git hash here)
export GIT_REF=dd2bfaf31fcd31b79b9f65e7fbc812338256bbd0

nohup docker/rosetta/docker-build-rosetta.sh  > /dev/null 2>&1 &

# Build data path for node & files
mkdir -p data

# Cp node config (and fix the paths)
cp config/src/config/test_data/public_full_node.yaml data/fullnode.yaml

# Download waypoint and genesis necessary to start a node
curl -s -o data/genesis.blob https://rosetta.aptosdev.com/genesis.blob
curl -s -o data/waypoint.txt https://rosetta.aptosdev.com/waypoint.txt

sleep 2400

# Run the node in online remote mode (detached mode)
nohup docker run -d -p 8082:8082 --rm -v $(pwd)/data:/opt/aptos aptos-core:rosetta-$GIT_REF online-remote --rest-api-url https://rosetta.aptosdev.com > /dev/null 2>&1 &

sleep 120

echo "download rosetta-cli"
curl -sSfL https://raw.githubusercontent.com/coinbase/rosetta-cli/master/scripts/install.sh | sh -s

echo "start check:data"
./bin/rosetta-cli --configuration-file crates/aptos-rosetta/rosetta_cli.json check:data --start-block $START_INDEX --end-block $END_INDEX
