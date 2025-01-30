#!/bin/bash
# Build all binaries and framework
cargo build --release -p aptos-node
cargo run --package aptos-framework release --target mainnet
cargo build --release -p aptos-faucet-service

mkdir "$GENESIS_DIR"
cp mainnet.mrb "$GENESIS_DIR/framework.mrb" 
cp target/release/aptos-node "$GENESIS_DIR/aptos-node"
cp target/release/aptos-faucet-service "$GENESIS_DIR/aptos-faucet-service"

# Generate layout file using yq
yq eval -n "
  .root_key = \"0x$ROOT_KEY\" |
  .users = [$(for i in $(seq 1 "$NODE_COUNT"); do echo -n "\"validator_$i\","; done | sed 's/,$//')]  |
  .chain_id = $CHAIN_ID |
  .allow_new_validators = false |
  .epoch_duration_secs = 7200 |
  .is_test = false |
  .min_price_per_gas_unit = 1 |
  .min_stake = 100000000000000 |
  .min_voting_threshold = 100000000000000 |
  .max_stake = 1000000000000000 |
  .recurring_lockup_duration_secs = 86400 |
  .required_proposer_stake = 1000000 |
  .rewards_apy_percentage = 10 |
  .voting_duration_secs = 43200 |
  .voting_power_increase_limit = 20
" > "$GENESIS_DIR/$LAYOUT_FILE"

# Generate keys and configs for each node
for i in $(seq 1 "$NODE_COUNT"); do
  VALIDATOR_DIR="${GENESIS_DIR}/validator_$i/keys"
  mkdir -p "$VALIDATOR_DIR"

  # Generate validator keys
  cargo run --package aptos -- genesis generate-keys --output-dir "$VALIDATOR_DIR"

  # Assign IP addresses
  # 10 is the offset for validator and 20 for fullnode, this is to avoid conflicts with the default docker bridge network and between the validator and fullnode
  VALIDATOR_IP="$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+10+'"$i"')}')"
  FULLNODE_IP="$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+20+'"$i"')}')"

  VALIDATOR_HOST="$VALIDATOR_IP:6180"
  FULLNODE_HOST="$FULLNODE_IP:6182"

  # Check if this node requires a full-node configuration
  if [[ " ${FULLNODE_NODES[@]} " =~ " $i " ]]; then
    FULLNODE_ARG="--full-node-host $FULLNODE_HOST"
  else
    FULLNODE_ARG=""
  fi

  # Generate ValidatorConfiguration (join during genesis already setup the stacking ammount for the validator so we don't need to faucet them)
  cargo run --package aptos -- genesis set-validator-configuration \
    --owner-public-identity-file "$VALIDATOR_DIR"/public-keys.yaml \
    --username "validator_$i" \
    --stake-amount 100000000000000 \
    --validator-host "$VALIDATOR_HOST" \
    $FULLNODE_ARG \
    --local-repository-dir .  \
    --join-during-genesis

    cp -r "validator_$i"/* "$GENESIS_DIR/validator_$i/"
    rm -rf "validator_$i"

    # Check if this node requires a full-node configuration
    if [[ " ${FULLNODE_NODES[@]} " =~ " $i " ]]; then
      
      # Read the required values from the generated files
      FULL_NODE_NETWORK_KEY=$(yq eval '.full_node_network_public_key' "$VALIDATOR_DIR/public-keys.yaml")
      ACCOUNT_ADDRESS=$(yq eval '.account_address' "$VALIDATOR_DIR/validator-full-node-identity.yaml")
      
      yq eval -n "
        .base.role = \"full_node\" |
        .base.data_dir = \"/opt/aptos/data\" |
        .base.waypoint.from_file = \"/opt/aptos/genesis/waypoint.txt\" |
        .execution.genesis_file_location = \"/opt/aptos/genesis/genesis.blob\" |
        .full_node_networks[0].network_id = \"public\" |
        .full_node_networks[0].discovery_method = \"onchain\" |
        .full_node_networks[0].identity.type = \"from_config\" |
        .full_node_networks[0].identity.key = \"$FULL_NODE_NETWORK_KEY\" |
        .full_node_networks[0].identity.peer_id = \"$ACCOUNT_ADDRESS\" |
        .full_node_networks[0].listen_address = \"/ip4/0.0.0.0/tcp/6182\" |
        .full_node_networks[0].max_inbound_connections = 100 |
        .full_node_networks[0].mutual_authentication = false |
        .full_node_networks[0].seed_addrs = {} |
        .storage.rocksdb_configs.enable_storage_sharding = true |
        .api.enabled = true |
        .api.address = \"0.0.0.0:8080\"
      " > "$GENESIS_DIR/validator_$i/validator.yaml"
      
    else
      yq eval -n "
        .base.role = \"validator\" |
        .base.data_dir = \"/opt/aptos/data\" |
        .base.waypoint.from_file = \"/opt/aptos/genesis/waypoint.txt\" |
        .consensus.safety_rules.service.type = \"local\" |
        .consensus.safety_rules.backend.type = \"on_disk_storage\" |
        .consensus.safety_rules.backend.path = \"secure-data.json\" |
        .consensus.safety_rules.backend.namespace = null |
        .consensus.safety_rules.initial_safety_rules_config.from_file.waypoint.from_file = \"/opt/aptos/genesis/waypoint.txt\" |
        .consensus.safety_rules.initial_safety_rules_config.from_file.identity_blob_path = \"/opt/aptos/genesis/validator-identity.yaml\" |
        .execution.genesis_file_location = \"/opt/aptos/genesis/genesis.blob\" |
        .storage.rocksdb_configs.enable_storage_sharding = true |
        .validator_network.discovery_method = \"onchain\" |
        .validator_network.mutual_authentication = true |
        .validator_network.identity.type = \"from_file\" |
        .validator_network.identity.path = \"/opt/aptos/genesis/validator-identity.yaml\" |
        .api.enabled = true |
        .api.address = \"0.0.0.0:8080\"
      " > "$GENESIS_DIR/validator_$i/validator.yaml"
    fi

done

cat "$GENESIS_DIR/layout.yaml"

# Generate genesis.blob and waypoint
cargo run --package aptos -- genesis generate-genesis --local-repository-dir "$GENESIS_DIR" --output-dir "$GENESIS_DIR"
