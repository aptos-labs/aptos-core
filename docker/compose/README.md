---
id: docker_compose
title: Aptos Docker-Compose Configuration
custom_edit_url: https://github.com/aptos/aptos/edit/main/docker/compose/README.md
---

This directory contains the following compose configurations:
* **validator-testnet**: creates a single validator test network, and a faucet that directly connects to it
* **public_full_node**: creates a public fullnode, and it can be configured to connect to any existing network (e.g. testnet, Mainnet).
* **monitoring**: creates a monitoring stack which can be used to collect metrics and virtulize it on a dashboard. This can be installed together with other compose configurations and provides simple monitoring for the deployment.
* **data-restore**: creates a DB restore job to restore a data volume from provided S3 bucket. This can be used to quickly restore fullnode for an existing blockchain to avoid spending long time on state-sync.

To use these compositions:
1. [Download](https://docs.docker.com/install/) and install Docker and Docker Compose (comes with Docker for Mac and Windows).
2. Open your favorite terminal and enter the appropriate composition directory
3. Run `docker-compose up`

To build your own complete testnet:
1. Start the **validator-testnet** and **faucet**:
    1. Enter the **validator-testnet** directory `cd validator-testnet`
    2. Start the composition `docker-compose up -d`
    3. Return to the compose directory: `cd ..`
 2. Enjoy your testnet:
    1. Faucet will be available at http://127.0.0.1:8000
    2. JSON-RPC will be available at http://127.0.0.1:8080

If you would like to clear the validator/blockchain data and start from scratch, either remove the docker volume `aptos-shared`,
or run `docker-compose run validator rm -rf '/opt/aptos/var/*'` from the **validator-testnet** directory.

To clear just the validator logs, run  `docker-compose run validator rm -rf '/opt/aptos/var/validator.log'`
