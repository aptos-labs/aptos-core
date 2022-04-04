---
title: "Run a local testnet"
slug: "run-a-local-testnet"
sidebar_position: 9
---
import BlockQuote from "@site/src/components/BlockQuote";

# Run a Local Testnet

You can run a local testnet of the Aptos Blockchain. This network runs independently of the Aptos ecosystem and is for testing and development purposes.

<BlockQuote type="info">
Note: your local testnet will not be connected to the Aptos devnet. It will run on your local machine and be contained there.
</BlockQuote>

## Getting Started

You can run a local testnet in two ways either using the Aptos-core source code or using Docker:
1. The Aptos-core source code is useful for testing modifications to the Aptos-core codebase or the Aptos Framework.
2. Docker is particularly useful for building services on top of the Aptos Blockchain or the Aptos Framework, as there is no build overhead and the ledger persists across network restarts (by default).

We describe each method below.

### Using the Aptos-core source code

1. Clone the Aptos-core repository from GitHub and prepare your developer environment by running the following commands:

    ```
    git clone https://github.com/aptos-labs/aptos-core.git
    cd aptos
    ./scripts/dev_setup.sh
    source ~/.cargo/env
    ```
2. Run the command: `cargo run -p aptos-node -- --test`. After starting up, the process should print its config path (e.g., `/private/var/folders/36/w0v54r116ls44q29wh8db0mh0000gn/T/f62a72f87940e3892a860c21b55b529b/0/node.yaml`) and other metadata.

Note: this command runs `aptos-node` from a genesis-only ledger state. If you want to reuse the ledger state produced by a previous run of `aptos-node`, use `cargo run -p aptos-node -- --test --config <config-path>`.

#### Attaching a Faucet to your Aptos Testnet

1. Start your local validator network
2. Copy the *Aptos root key path* and use it to replace the `mint-key-file-path` below 
3. Run the following command to start a Faucet: ```
   cargo run --package aptos-faucet -- \
      --chain-id TESTING \
      --mint-key-file-path "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/mint.key" \
      --address 0.0.0.0 \
      --port 8000 \
      --server-url http://127.0.0.1:8080
```

This will start a Faucet running locally without any restrictions to tokens that can be claimed / minted and make the serivce as accessible as the testnet started above. Faucets are stateless services that can be run in parallel.

### Using Docker

1. Install [Docker](docker) including Docker-Compose.
2. Create a directory for your local test validator network.
3. Download the [validator testnet docker compose](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/validator-testnet/docker-compose.yaml) and [validator configuration](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/validator-testnet/validator_node_template.yaml).
4. Start Docker-Compose `docker-compose up`

This will start both a Validator and a Faucet. The Validator's REST endpoint will be avilable at `http://127.0.0.1:8080` and the Faucet at `http://127.0.0.1:8000`.

As the software is in the early stages of development, it is worth noting that there may be breaking changes. If the software fails to start, delete both the containers and shared volumes, which can be queried via `docker container ls -a` and `docker volume ls` and deleted via `docker container rm $id` and `docker volume rm $name`.

If you intend to use your Testnet over an extended period of time, you should pin the images to a specific ID. Image ID's can be obtained via `docker container ls` and added to the docker compose file.


## Interacting with the local test validator network
After starting your local test validator network, you should see the following:

```
Entering test mode, this should never be used in production!
Completed generating configuration:
        Log file: "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/validator.log"
        Config path: "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/0/node.yaml"
        Aptos root key path: "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/mint.key"
        Waypoint: 0:197bc8b76761622c2d2054d8bf93c1802fa0eb4bc55f0f3d4442878fdecc297f
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        FullNode network: /ip4/0.0.0.0/tcp/7180

Aptos is running, press ctrl-c to exit
```

This output contains information required for starting the Aptos CLI tool:
* `Aptos root key path`: The root key (also known as the mint or faucet key) controls the account that can mint tokens. Available in the docker compose folder under `aptos_root_key`.
* `Waypoint`: A verifiable checkpoint of the blockchain (available in the docker compose folder under waypoint.txt)
* `REST endpoint`: The endpoint for the REST service, e.g., `http://127.0.0.1:8080`.
* `ChainId`: The chain id uniquely distinguishes this network from other blockchain networks.

## Next Steps

At this point, you will have a special root account at `0x1` that can perform the mint operation. Follow up with: 

* [Your first transaction](/tutorials/your-first-transaction) to learn how to submit transactions.
* [Your first Move module](/tutorials/your-first-move-module) to learn how to create Move modules.
* [Interacting with the Aptos Blockchain](/transactions/interacting-with-the-aptos-blockchain) to learn how to mint coins.

It is important to note that this guide does not include creating a faucet. That is left as an exercise for the reader.

[docker](https://docs.docker.com/get-docker/)
