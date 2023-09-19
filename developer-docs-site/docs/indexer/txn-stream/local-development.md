---
title: "Running Locally"
---

# Running the Transaction Stream Service Locally

:::info
This has been tested on MacOS 13 on ARM and Debian 11 on x86_64.
:::

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
- Python 3.8+: [Installation Guide](https://docs.python-guide.org/starting/installation/#python-3-installation-guides).
- Poetry: [Installation Guide](https://python-poetry.org/docs/#installation).
- Docker: [Installation Guide](https://docs.docker.com/get-docker/).
- Docker Compose v2: This should be installed by default with modern Docker installations, verify with this command:
```bash
docker-compose version --short
```
- grpcurl: [Installation Guide](https://github.com/fullstorydev/grpcurl#installation)
- OpenSSL

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
When you start, stop, and start the service again, it will re-use the same local testnet data. If you wish to wipe the local testnet and start from scratch you can run the following command:
```
poetry run python indexer_grpc_local.py wipe
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

## Debugging

### Usage on ARM systems
If you have a machine with an ARM processor, e.g. an M1/M2 Mac, the script should detect that and set the appropriate environment variables to ensure that the correct images will be used. If you have issues with this, try setting the following environment variable:
```bash
export DOCKER_DEFAULT_PLATFORM=linux/amd64
```

Additionally, make sure the following settings are correct in Docker Desktop:
- Enabled: Preferences > General > Use Virtualization framework
- Enabled: Preferences > General > Use Docker Compose V2
- Disabled: Features in development -> Use Rosetta for x86/amd64 emulation on Apple Silicon

This script has not been tested on Linux ARM systems.

### Redis fails to start
Try setting the following environment variable before running the script:
```bash
export REDIS_IMAGE_REPO=arm64v8/redis
```

### Cache worker is crashlooping or `Redis latest version update failed.` in log
Wipe the data:
```bash
poetry run python indexer_grpc_local.py wipe
```

This means historical data will be lost.
