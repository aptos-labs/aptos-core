#!/bin/bash -xe

TEMP="$(mktemp -d)"
SOURCE_DIR="$(dirname "$BASH_SOURCE")"

mkdir "$TEMP"/keys
echo > "$TEMP"/keys/validator-identity.yaml

cp "$SOURCE_DIR"/docker-compose.yaml "$TEMP"
cp "$SOURCE_DIR"/validator.yaml "$TEMP"
cp "$SOURCE_DIR"/haproxy.cfg "$TEMP"
cp "$SOURCE_DIR"/blocked.ips "$TEMP"

cd "$TEMP"

# This doesnt actually need to work, just open the ports
wget https://github.com/aptos-labs/aptos-networks/raw/main/devnet/genesis.blob
wget https://github.com/aptos-labs/aptos-networks/raw/main/devnet/waypoint.txt

docker compose up -d

# Magically shut down docker at the end of the script even if it errors
function shutdown {
    docker compose down
}
trap shutdown EXIT

sleep 10

curl localhost:9101/metrics &>/dev/null
curl localhost:9102/metrics &>/dev/null
curl localhost:9102/stats &>/dev/null

nc -zv localhost 6180
nc -zv localhost 6181