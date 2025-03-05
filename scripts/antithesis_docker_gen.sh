#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

# Default variables
LAYOUT_FILE="layout.yaml"
GENESIS_DIR="genesis_antithesis"
CHAIN_ID=8
ROOT_KEY=""
NETWORK_IP="192.168.5.0"
NODE_COUNT=4
FULLNODE_NODES=()
APTOS_BRANCH="main"
# Print usage
function usage() {
  echo "Usage: $0 [options]"
  echo "Options:"
  echo "  -b APTOS_BRANCH        Aptos branch (default: $APTOS_BRANCH)"
  echo "  -d GENESIS_DIR         Directory for genesis (default: $GENESIS_DIR)"
  echo "  -r ROOT_KEY            Root key for genesis (default new key)"
  echo "  -c CHAIN_ID            Chain ID (default: $CHAIN_ID)"
  echo "  -i NETWORK_IP          Network IP address for nodes (default: $NETWORK_IP)"
  echo "  -n NODE_COUNT          Number of nodes (default: $NODE_COUNT)"
  echo "  -x FULLNODE_NODES      Comma-separated list of node indices to include full-node config (e.g., 1,3)"
  echo "  -h                     Show this help message"
}

# Parse command-line arguments
while getopts "b:d:r:c:i:n:x:h" opt; do
  case $opt in
    b) APTOS_BRANCH="$OPTARG" ;;
    d) GENESIS_DIR="$OPTARG" ;;
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
    #     docker container ls -a | grep "genesis_antithesis-${type}" | awk '{print $1}' | xargs -r docker rm
    #     docker image ls | grep "genesis_antithesis-${type}" | awk '{print $3}' | xargs -r docker rmi
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

# Add version parsing after APTOS_BRANCH is set
DOCKER_TAG="latest"
if [[ $APTOS_BRANCH =~ ^aptos-node-v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
    DOCKER_TAG="${BASH_REMATCH[1]}"
    echo "Using version $DOCKER_TAG from branch $APTOS_BRANCH"
else
    echo "Using default 'latest' tag for docker images"
fi

# Build the Aptos framework, node, genesis and all the configs and identities
echo "Building Aptos framework, node and genesis, it may take a while..."
docker build \
    -t genesis_antithesis-build \
    -f scripts/antithesis-templates/Dockerfile-build \
    --build-arg APTOS_BRANCH="$APTOS_BRANCH" \
    --build-arg NODE_COUNT="$NODE_COUNT" \
    --build-arg NETWORK_IP="$NETWORK_IP" \
    --build-arg FULLNODE_NODES="${FULLNODE_NODES[*]}" \
    --build-arg ROOT_KEY="$ROOT_KEY" \
    --build-arg CHAIN_ID="$CHAIN_ID" \
    --build-arg GENESIS_DIR=$GENESIS_DIR \
    --build-arg LAYOUT_FILE=$LAYOUT_FILE \
    .
docker create --name genesis_antithesis-builder genesis_antithesis-build
docker cp genesis_antithesis-builder:/aptos-core/$GENESIS_DIR/. $GENESIS_DIR/
docker rm -f genesis_antithesis-builder
echo "Genesis blob and waypoint generated in $GENESIS_DIR."

# Fetch docker images (those images need to be available in the antithesis infra, so retag them and push them)
docker pull postgres:14
docker pull aptos/aptos-indexer:latest
docker pull hasura/graphql-engine:v2.36.0
echo "!!! Please push the following images to the antithesis infra !!!"
echo "postgres:14"
echo "aptos/aptos-indexer:latest"
echo "hasura/graphql-engine:v2.36.0"

# Entrypoint and Dockerfile copy
cp scripts/antithesis-templates/entrypoint.sh $GENESIS_DIR/entrypoint.sh
cp scripts/antithesis-templates/healthcheck.sh $GENESIS_DIR/healthcheck.sh
cp -r scripts/antithesis-templates/antithesis-tests $GENESIS_DIR/antithesis-tests
cp scripts/antithesis-templates/hasura_metadata.json $GENESIS_DIR/hasura_metadata.json
#awk -v ip="$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+30)}')" '{gsub("<FAUCET_IP>", ip); print}' scripts/antithesis-templates/singleton_driver_test1.sh > $GENESIS_DIR/singleton_driver_test1.sh
cp scripts/antithesis-templates/Dockerfile-node $GENESIS_DIR/Dockerfile-node
cp scripts/antithesis-templates/Dockerfile-config $GENESIS_DIR/Dockerfile-config
cp scripts/antithesis-templates/Dockerfile-client $GENESIS_DIR/Dockerfile-client

# Generate docker-compose.yaml using yq
SERVICES=""

# Generate indexer configs
processor=("account_transaction_processor" "events_processor" "fungible_asset_processor" "objects_processor" "token_v2_processor" "transaction_metadata_processor" "user_transaction_processor" "default_processor")
counter=0
for i in "${processor[@]}"; do
  echo "processor: $i"
    SERVICES="$SERVICES .services.indexer_$i.image = \"indexer-processor-rust:latest\" |"
    SERVICES="$SERVICES .services.indexer_$i.container_name = \"aptos-indexer-processor-$i\" |"
    SERVICES="$SERVICES .services.indexer_$i.hostname = \"aptos-indexer-processor-$i\" |"
    SERVICES="$SERVICES .services.indexer_$i.environment.RUST_LOG = \"error\" |"
    if [ "$counter" -eq 0 ]; then
      SERVICES="$SERVICES .services.indexer_$i.depends_on.aptos-indexer-postgres.condition = \"service_healthy\" |"
    else
      SERVICES="$SERVICES .services.indexer_$i.depends_on.aptos-indexer-processor-${processor[$counter-1]}.condition = \"service_started\" |"
    fi
    SERVICES="$SERVICES .services.indexer_$i.command = \"-c processor-config.yaml\" |"
    SERVICES="$SERVICES .services.indexer_$i.volumes = [\"./indexer/processor-config-$i.yaml:/opt/aptos/etc/processor-config.yaml\"] |"
    SERVICES="$SERVICES .services.indexer_$i.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+70+counter)}')\" |"
    SERVICES="$SERVICES .services.indexer_$i.restart = \"unless-stopped\" |"
    SERVICES="$SERVICES .services.indexer_$i.expose = [8090] |"
    ((counter++))
done

# Hasura API
SERVICES="$SERVICES .services.migration-check.image = \"postgres:14\" |"
SERVICES="$SERVICES .services.migration-check.container_name = \"migration-check\" |"
SERVICES="$SERVICES .services.migration-check.hostname = \"migration-check\" |"
SERVICES="$SERVICES .services.migration-check.command = \"bash -c 'echo \"Waiting for migrations to complete...\" && while true; do echo \"\\nChecking migration status...\" && result=\$(timeout 10s PGPASSWORD= psql -h 192.168.5.71 -p 5432 -U postgres -d local_testnet -t -c \"SELECT * FROM transactions LIMIT 1\" 2>&1 || echo \"TIMEOUT\") && if [ \"\$result\" = \"TIMEOUT\" ]; then echo \"Connection timed out, retrying in 5 seconds...\" && sleep 5 && continue; fi && echo \"Transaction query result:\" && psql -h 192.168.5.71 -p 5432 -U postgres -d local_testnet -t -c \"SELECT * FROM transactions LIMIT 1\" && break; done'\" |"
SERVICES="$SERVICES .services.migration-check.depends_on.aptos-indexer-postgres.condition = \"service_healthy\" |"
SERVICES="$SERVICES .services.migration-check.depends_on.aptos-indexer-processor-${processor[$counter]}.condition = \"service_started\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.image = \"graphql-engine:v2.36.0\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.container_name = \"aptos-indexer-api\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.environment.PG_DATABASE_URL = \"postgres://postgres@$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+71)}'):5432/local_testnet\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.environment.HASURA_GRAPHQL_METADATA_DATABASE_URL = \"postgres://postgres@$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+71)}'):5432/local_testnet\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.environment.INDEXER_V2_POSTGRES_URL = \"postgres://postgres@$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+71)}'):5432/local_testnet\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.environment.HASURA_GRAPHQL_DEV_MODE = \"true\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.environment.HASURA_GRAPHQL_ENABLE_CONSOLE = \"true\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.environment.HASURA_GRAPHQL_CONSOLE_ASSETS_DIR = \"/srv/console-assets\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.environment.HASURA_GRAPHQL_SERVER_PORT = \"8090\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.depends_on.migration-check.condition = \"service_completed_successfully\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+70)}')\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.restart = \"unless-stopped\" |"
SERVICES="$SERVICES .services.aptos-indexer-hasura.expose = [8090] |"


for i in $(seq 1 "$NODE_COUNT"); do
    if [ "$i" -gt 1 ]; then
        SERVICES="$SERVICES |"
    fi
    SERVICES="$SERVICES .services.validator_$i.image = \"aptos-node:$DOCKER_TAG\" |"
    SERVICES="$SERVICES .services.validator_$i.container_name = \"validator_$i\" |"
    SERVICES="$SERVICES .services.validator_$i.hostname = \"validator_$i\" |"
    SERVICES="$SERVICES .services.validator_$i.environment.ROLE = \"validator\" |"
    SERVICES="$SERVICES .services.validator_$i.environment.CONFIG_PATH = \"/opt/aptos/etc/validator.yaml\" |"
    SERVICES="$SERVICES .services.validator_$i.volumes = [\"./validator_$i/validator.yaml:/opt/aptos/etc/validator.yaml\", \"./validator_$i/keys/validator-identity.yaml:/opt/aptos/genesis/validator-identity.yaml\"] |"
    SERVICES="$SERVICES .services.validator_$i.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+10+'"$i"')}')\" |"
    SERVICES="$SERVICES .services.validator_$i.restart = \"unless-stopped\" |"
    SERVICES="$SERVICES .services.validator_$i.expose = [6180, 6181, 9101, 8080]"

    if [[ "${FULLNODE_NODES[*]}" =~ "$i" ]]; then
      SERVICES="$SERVICES |"
      SERVICES="$SERVICES .services.fullnode_$i.image = \"aptos-node:$DOCKER_TAG\" |"
      SERVICES="$SERVICES .services.fullnode_$i.container_name = \"fullnode_$i\" |"
      SERVICES="$SERVICES .services.fullnode_$i.hostname = \"fullnode_$i\" |"
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
  .services.faucet.image = \"aptos-node:$DOCKER_TAG\" |
  .services.faucet.container_name = \"faucet\" |
  .services.faucet.hostname = \"faucet\" |
  .services.faucet.environment.ROLE = \"faucet\" |
  .services.faucet.environment.MINT_KEY = \"/opt/aptos/etc/mint.key\" |
  .services.faucet.environment.NODE_URL = \"http://$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+11)}'):8080\" |
  .services.faucet.environment.CHAIN_ID = \"$CHAIN_ID\" |
  .services.faucet.volumes = [\"./mint.key:/opt/aptos/etc/mint.key\"] |
  .services.faucet.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+30)}')\" |
  .services.faucet.restart = \"unless-stopped\" |
  .services.faucet.expose = [8081] |
  .services.healthcheck.image = \"aptos-node:$DOCKER_TAG\" |
  .services.healthcheck.container_name = \"healthcheck\" |
  .services.healthcheck.hostname = \"healthcheck\" |
  .services.healthcheck.environment.ROLE = \"healthcheck\" |
  .services.healthcheck.environment.NODE_COUNT = \"$NODE_COUNT\" |
  .services.healthcheck.environment.NETWORK_IP = \"$NETWORK_IP\" |
  .services.healthcheck.environment.NODE_URL = \"http://$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+11)}'):8080\" |
  .services.healthcheck.environment.INDEXER_URL = \"http://$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+70)}'):8090\" |
  .services.healthcheck.volumes = [\"./healthcheck.sh:/usr/local/bin/healthcheck.sh\"] |
  .services.healthcheck.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+50)}')\" |
  .services.client.image = \"aptos-client:$DOCKER_TAG\" |
  .services.client.container_name = \"client\" |
  .services.client.hostname = \"client\" |
  .services.client.environment.FAUCET_URL = \"http://$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+30)}'):8081\" |
  .services.client.environment.FULLNODE_URL = \"http://$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+11)}'):8080/v1\" |
  .services.client.environment.APTOS_NETWORK = \"local\" |
  .services.client.environment.NETWORK_IP = \"$NETWORK_IP\" |
  .services.client.environment.CHAIN_ID = \"$CHAIN_ID\" |
  .services.client.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+60)}')\" |
  .services.client.restart = \"unless-stopped\" |
  .networks.custom_network.driver = \"bridge\" |
  .networks.custom_network.ipam.config[0].subnet = \"$NETWORK_IP/24\" |
  .services.aptos-indexer-postgres.image = \"postgres:14\" |
  .services.aptos-indexer-postgres.container_name = \"aptos-indexer-postgres\" |
  .services.aptos-indexer-postgres.hostname = \"aptos-indexer-postgres\" |
  .services.aptos-indexer-postgres.environment.POSTGRES_USER = \"postgres\" |
  .services.aptos-indexer-postgres.environment.POSTGRES_HOST_AUTH_METHOD = \"trust\" |
  .services.aptos-indexer-postgres.environment.POSTGRES_DB = \"local_testnet\" |
  .services.aptos-indexer-postgres.healthcheck.test = [\"CMD-SHELL\", \"pg_isready -U postgres\"] |
  .services.aptos-indexer-postgres.healthcheck.interval = \"5s\" |
  .services.aptos-indexer-postgres.healthcheck.timeout = \"5s\" |
  .services.aptos-indexer-postgres.healthcheck.retries = 5 |
  .services.aptos-indexer-postgres.restart = \"unless-stopped\" |
  .services.aptos-indexer-postgres.expose = [5432] |
  .services.aptos-indexer-postgres.networks.custom_network.ipv4_address = \"$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+71)}')\" |
  $SERVICES
" > "$GENESIS_DIR/docker-compose.yml"

wget https://storage.googleapis.com/antithesis-aptos/libvoidstar.so -O $GENESIS_DIR/libvoidstar.so

echo "Enter $GENESIS_DIR and run:"
echo "docker build -f Dockerfile-node -t aptos-node:$DOCKER_TAG ."
echo "docker build -f Dockerfile-config -t config:$DOCKER_TAG ."
echo "docker build -f Dockerfile-client -t aptos-client:$DOCKER_TAG ."
echo "If you want to test locally run: docker compose up --force-recreate"
echo "Then push images to antithesis infra"