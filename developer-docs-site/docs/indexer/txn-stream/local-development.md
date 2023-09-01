---
title: "Running Locally"
---

# Running the Transaction Stream Service Locally

When building a custom processor, you might find it helpful to develop against a local development stack. The Transaction Stream Service is a complicated, multi-component system. To assist with local development, we offer a Python script that wraps a Docker compose file to set up the entire system.

This script sets up the following:
- Single node testnet with the indexer GRPC stream enabled.
- A Redis instance.
- Transaction Stream Service, including the following components:
  - [cache-worker](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/indexer-grpc/indexer-grpc-cache-worker): Pulls transactions from the node and stores them in Redis.
  - [file-store](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/indexer-grpc/indexer-grpc-file-store): Fetches transactions from Redis and stores them in a filesystem.
  - [data-service](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/indexer-grpc/indexer-grpc-data-service): Serves transactions via a GRPC stream to downstream clients. It pulls from either the cache or the file store depending on the age of the transaction.
- Shared volumes and networking to hook it all up.

You can learn more about the Transaction Stream Service architecture [here](/indexer/txn-stream) and the Docker compose file [here](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/indexer-grpc/docker-compose.yaml).

## Prerequisites
In order to use the local development script you must have the following installed:
- Python 3.7+: [Installation Guide](https://docs.python-guide.org/starting/installation/#python-3-installation-guides).
- Poetry: [Installation Guide](https://python-poetry.org/docs/#installation).
- Docker: [Installation Guide](https://docs.docker.com/get-docker/).

Make sure Docker is running.

## Preparation
Clone the aptos-core repo:
```
# HTTPS
git clone https://github.com/aptos-labs/aptos-core.git

# SSH
git clone git@github.com:aptos-labs/aptos-core.git
```

Navigate to the `testsuite` directory:
```
cd aptos-core
cd testsuite
```

Install the Python dependencies:
```
poetry install
```

## Running the script
### Starting the service
```
poetry run python indexer_grpc_local.py start
```

You will know this succeeded if the command exits and you see the following:
```
Attempting to stream from indexer grpc for 10s
Stream finished successfully
```

### Stopping the service
```
poetry run python indexer_grpc_local.py stop
```

### Wiping the data
When you start, stop, and start the service again, it will re-use the same local testnet data. If you wish to wipe the local testnet and start from scratch, you can run the following command:
```
poetry run python indexer_grpc_local.py wipe
```

### Usage on ARM systems
If you have a machine with an ARM processor, e.g. an M1/M2 Mac, you should set the following environment variable before running any of these commands:
```
DOCKER_DEFAULT_PLATFORM=linux/amd64
```

## Using the local service
You can connect to the local Transaction Stream Service, e.g. from a custom processor, using the following configuration values:
```
indexer_grpc_data_service_address: 127.0.0.1:50052
auth_token: dummy_token
```

You can connect to the node at the following address:
```
http://127.0.0.1:8080/v1
```
