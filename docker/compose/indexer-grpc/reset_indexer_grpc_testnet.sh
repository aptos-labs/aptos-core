#!/bin/bash

# kill everything
docker ps -a | grep -E "validator|faucet|indexer|redis" | awk '{ print $1 }' | xargs -I{} docker kill {}
docker ps -a | grep -E "validator|faucet|indexer|redis" | awk '{ print $1 }' | xargs -I{} docker rm {}

# delete volume
docker volume rm aptos-shared indexer-grpc-file-store
