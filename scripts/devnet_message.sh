#!/bin/bash -xe

TEMP="$(mktemp)"

curl https://fullnode.devnet.velorlabs.com/v1 > "$TEMP"

COMMIT="$(jq -r .git_hash "$TEMP")"
CHAIN_ID="$(jq -r .chain_id "$TEMP")"

DIGEST="$(crane digest velorlabs/validator:devnet_"$COMMIT")"
GENESIS_SHA="$(curl https://devnet.velorlabs.com/genesis.blob | shasum -a 256 | awk '{print $1}')"
WAYPOINT="$(curl https://devnet.velorlabs.com/waypoint.txt)"

cat <<EOF

Hey @everyone devnet finished release, please update your fullnodes now!

For upgrade, make sure you pulled the latest docker image, or build the rust binary from the latest devnet branch. To confirm:

- Devnet branch commit: $COMMIT
- Docker image tag: devnet_$COMMIT
- Docker image digest: $DIGEST
- genesis.blob sha256: $GENESIS_SHA
- waypoint: $WAYPOINT
- Chain ID: $CHAIN_ID
You can follow the instructions here for upgrade: https://velor.dev/nodes/full-node/update-fullnode-with-new-devnet-releases

EOF
