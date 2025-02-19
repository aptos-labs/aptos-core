#!/bin/bash

NODE_COUNT=$1
NETWORK_IP=$2

if [ -z "$ANTITHESIS_OUTPUT_DIR" ]; then
  echo "ANTITHESIS_OUTPUT_DIR is not set"
  ANTITHESIS_OUTPUT_DIR="/tmp/"
fi

echo "Starting aptos-healthcheck loop"

while true; do
  HEALTHY_NODES=()
  for NODE in $(seq 1 "$NODE_COUNT"); do
    echo "Checking health of $NODE"
    IP=$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+10+'"$NODE"')}')
    ROUND=$(curl "http://$IP:9101/metrics" 2> /dev/null | grep "^aptos_consensus_current_round " | awk '{print $2}')
    echo "Round $NODE: $ROUND"
    
    if [ -n "$ROUND" ] && [ "$ROUND" -gt 1 ]; then
      if [[ ! " ${HEALTHY_NODES[*]} " =~ ${IP} ]]; then
        echo "Adding $IP to healthy nodes"
        HEALTHY_NODES+=("$IP")
      fi
    fi
  done

  /usr/local/bin/aptos init --profile test1 --rest-url "${NODE_URL}" --faucet-url "http://$(echo "$NETWORK_IP" | awk -F '.' '{print $1"."$2"."$3"."($4+30)}'):8081" --network custom --assume-yes
  FOUNDING_SUCCESS=$?

  if [ $FOUNDING_SUCCESS -eq 0 ] && [ ${#HEALTHY_NODES[@]} -eq "$NODE_COUNT" ]; then
    echo "All nodes are healthy!"
    #JSONL message to sdk.jsonl
    echo '{"antithesis_setup": { "status": "complete", "details": null }}' >> "$ANTITHESIS_OUTPUT_DIR/sdk.jsonl"
    exit 0
  fi
  
  sleep 5
done
