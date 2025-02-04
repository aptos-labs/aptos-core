#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

# Default variables
LAYOUT_FILE="layout.yaml"
GENESIS_DIR="genesis_anthitesis"
CHAIN_ID=8
ROOT_KEY=""
NETWORK_IP="127.0.0.0"
NODE_COUNT=1
FULLNODE_NODES=()
APTOS_BRANCH="main"
# Print usage
function usage() {
  echo "Usage: $0 [options]"
  echo "Options:"
  echo "  -b APTOS_BRANCH        Aptos branch (default: $APTOS_BRANCH)"
  echo "  -f FRAMEWORK_DIR       Directory for Aptos framework release (default: $FRAMEWORK_DIR)"
  echo "  -r ROOT_KEY            Root key for genesis (default: $ROOT_KEY)"
  echo "  -c CHAIN_ID            Chain ID (default: $CHAIN_ID)"
  echo "  -i NETWORK_IP          Network IP address for nodes (default: $NETWORK_IP)"
  echo "  -n NODE_COUNT          Number of nodes (default: $NODE_COUNT)"
  echo "  -x FULLNODE_NODES      Comma-separated list of node indices to include full-node config (e.g., 1,3)"
  echo "  -h                     Show this help message"
}

# Parse command-line arguments
while getopts "b:f:r:c:i:n:x:h" opt; do
  case $opt in
    b) APTOS_BRANCH="$OPTARG" ;;
    f) FRAMEWORK_DIR="$OPTARG" ;;
    r) ROOT_KEY="$OPTARG" ;;
    c) CHAIN_ID="$OPTARG" ;;
    i) NETWORK_IP="$OPTARG" ;;
    n) NODE_COUNT="$OPTARG" ;;
    x) IFS=',' read -ra FULLNODE_NODES <<< "$OPTARG" ;;
    h) usage; exit 0 ;;
    *) usage; exit 1 ;;
  esac
done

if ! command -v yq &> /dev/null; then
    echo "yq is required to be installed. Please install yq and try again."
    exit 1
fi

if [ -d "$GENESIS_DIR" ]; then
    echo "Removing existing $GENESIS_DIR directory..."
    rm -rf $GENESIS_DIR
    echo "Cleaning up docker images and containers..."
    # for type in validator_ fullnode_ build faucet; do
    #     docker container ls -a | grep "genesis_anthitesis-${type}" | awk '{print $1}' | xargs -r docker rm
    #     docker image ls | grep "genesis_anthitesis-${type}" | awk '{print $3}' | xargs -r docker rmi
    # done
fi
mkdir -p $GENESIS_DIR

if [ -z "$ROOT_KEY" ]; then
      echo "Generating root key..."
      aptos key generate --key-type ed25519 --output-file $GENESIS_DIR/mint.key --encoding hex
      ROOT_KEY=$(cat $GENESIS_DIR/mint.key.pub)
      echo "Root key generated: $ROOT_KEY"
    else
      echo "Using provided public root key: $ROOT_KEY"
      echo "Please provide the private root key in the $GENESIS_DIR/mint.key file before running docker-compose up"
fi

# Build the Aptos framework, node, genesis and all the configs and identities
echo "Building Aptos framework, node and genesis, it may take a while..."
docker build \
    -t genesis_anthitesis-build \
    -f scripts/anthitesis-templates/Dockerfile-build \
    --build-arg APTOS_BRANCH="$APTOS_BRANCH" \
    --build-arg NODE_COUNT="$NODE_COUNT" \
    --build-arg NETWORK_IP="$NETWORK_IP" \
    --build-arg FULLNODE_NODES="${FULLNODE_NODES[*]}" \
    --build-arg ROOT_KEY="$ROOT_KEY" \
    --build-arg CHAIN_ID="$CHAIN_ID" \
    --build-arg GENESIS_DIR=$GENESIS_DIR \
    --build-arg LAYOUT_FILE=$LAYOUT_FILE \
    .
docker create --name genesis_anthitesis-builder genesis_anthitesis-build
docker cp genesis_anthitesis-builder:/aptos-core/$GENESIS_DIR/. $GENESIS_DIR/
docker rm -f genesis_anthitesis-builder
echo "Genesis blob and waypoint generated in $GENESIS_DIR."

# Entrypoint and Dockerfile copy
cp scripts/anthitesis-templates/entrypoint.sh $GENESIS_DIR/entrypoint.sh
cp scripts/anthitesis-templates/healthcheck.sh $GENESIS_DIR/healthcheck.sh
cp scripts/anthitesis-templates/Dockerfile $GENESIS_DIR/Dockerfile
cp scripts/anthitesis-templates/Dockerfile-config $GENESIS_DIR/Dockerfile-config

# Generate docker-compose.yaml using yq
SERVICES=""
for i in $(seq 1 "$NODE_COUNT"); do
    if [ "$i" -gt 1 ]; then
        SERVICES="$SERVICES |"
    fi
    SERVICES="$SERVICES .services.validator_$i.image = \"aptos/node:latest\" |"
    SERVICES="$SERVICES .services.validator_$i.environment.ROLE = \"validator\" |"
    SERVICES="$SERVICES .services.validator_$i.environment.CONFIG_PATH = \"/opt/aptos/etc/validator.yaml\" |"
    SERVICES="$SERVICES .services.validator_$i.volumes = [\"./validator_$i/validator.yaml:/opt/aptos/etc/validator.yaml\", \"./validator_$i/keys/validator-identity.yaml:/opt/aptos/genesis/validator-identity.yaml\"] |"
    SERVICES="$SERVICES .services.validator_$i.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+10+'"$i"')}')\" |"
    SERVICES="$SERVICES .services.validator_$i.restart = \"unless-stopped\" |"
    SERVICES="$SERVICES .services.validator_$i.expose = [6180, 6181, 9101, 8080]"

    if [[ "${FULLNODE_NODES[*]}" =~ "$i" ]]; then
      SERVICES="$SERVICES |"
      SERVICES="$SERVICES .services.fullnode_$i.image = \"aptos/node:latest\" |"
      SERVICES="$SERVICES .services.fullnode_$i.environment.ROLE = \"full_node\" |"
      SERVICES="$SERVICES .services.fullnode_$i.environment.CONFIG_PATH = \"/opt/aptos/etc/fullnode.yaml\" |"
      SERVICES="$SERVICES .services.fullnode_$i.volumes = [\"./validator_$i/fullnode.yaml:/opt/aptos/etc/fullnode.yaml\", \"./validator_$i/keys/validator-full-node-identity.yaml:/opt/aptos/genesis/validator-full-node-identity.yaml\"] |"
      SERVICES="$SERVICES .services.fullnode_$i.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+20+'"$i"')}')\" |"
      SERVICES="$SERVICES .services.fullnode_$i.restart = \"unless-stopped\" |"
      SERVICES="$SERVICES .services.fullnode_$i.expose = [6182, 8080, 9101]"
    fi
done


# NODE_URL is the first validator node for the faucet to connect to
yq eval -n "
  .services.faucet.image = \"aptos/node:latest\" |
  .services.faucet.environment.ROLE = \"faucet\" |
  .services.faucet.environment.MINT_KEY = \"/opt/aptos/etc/mint.key\" |
  .services.faucet.environment.NODE_URL = \"http://$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+11)}'):8080\" |
  .services.faucet.environment.CHAIN_ID = \"$CHAIN_ID\" |
  .services.faucet.volumes = [\"./mint.key:/opt/aptos/etc/mint.key\"] |
  .services.faucet.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+30)}')\" |
  .services.faucet.restart = \"unless-stopped\" |
  .services.faucet.expose = [8080] |
  .services.healthcheck.image = \"aptos/node:latest\" |
  .services.healthcheck.environment.ROLE = \"healthcheck\" |
  .services.healthcheck.environment.NODE_COUNT = \"$NODE_COUNT\" |
  .services.healthcheck.environment.NETWORK_IP = \"$NETWORK_IP\" |
  .services.healthcheck.volumes = [\"./healthcheck.sh:/usr/local/bin/healthcheck.sh\"] |
  .services.healthcheck.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+50)}')\" |
  .networks.custom_network.driver = \"bridge\" |
  .networks.custom_network.ipam.config[0].subnet = \"$NETWORK_IP/24\" |
  $SERVICES
" > "$GENESIS_DIR/docker-compose.yaml"

wget https://storage.googleapis.com/anthithesis/libvoidstar.so -O $GENESIS_DIR/libvoidstar.so

echo "Enter $GENESIS_DIR and run docker build -t aptos/node:latest . && docker compose up --force-recreate"
echo "If you want to upload on the infra run: docker build -f Dockerfile-config -t aptos/config:latest ." 