---
title: "Run a local testnet"
slug: "run-a-local-testnet"
sidebar_position: 9
---
import BlockQuote from "@site/src/components/BlockQuote";

# Run a Local Testnet

You can run a local testnet to test and develop against an Aptos Blockchain. This network runs independently of the Aptos ecosystem and is for testing and development purposes only.

<BlockQuote type="info">
Your local testnet will not be connected Aptos devnet.
</BlockQuote>

## Getting Started

You can run a local testnet in two ways: using the Aptos-core source code or Docker. The Aptos-core source code is useful when testing modifications to the Aptos-core code base or Aptos Framework. Docker is particularly useful when building services on top of the Aptos Blockchain or applications on top of the Aptos Framework as there is no build overhead and the ledger state persists across restarts of the network by default.

### Using Aptos-core source code

1. Download and clone the Aptos-core repository from GitHub and prepare your developer environment by running the following commands:

    ```
    git clone https://github.com/aptos-labs/aptos-core.git
    cd aptos
    ./scripts/dev_setup.sh
    source ~/.cargo/env
    ```
2. Run the process: `cargo run -p aptos-node -- --test`. After starting up, the process should print its config path (e.g., `/private/var/folders/36/w0v54r116ls44q29wh8db0mh0000gn/T/f62a72f87940e3892a860c21b55b529b/0/node.yaml`) and other metadata.

Note: this command runs `aptos-node` from a genesis-only ledger state. If you want to reuse the ledger state produced by a previous run of `aptos-node`, use `cargo run -p aptos-node -- --test --config <config-path>`.

### Using Docker

1. Install Docker and Docker-Compose.
2. Create a directory for your local test validator network.
3. Download the [validator testnet docker compose](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/validator-testnet/docker-compose.yaml) and [validator configuration](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/validator-testnet/validator_node_template.yaml).
4. Create configuration files in the same directory so that the data can be exported out of the docker container:
    ```
    # Linux / Mac
    touch genesis.blob aptos_root_key waypoint.txt

    # Windows
    fsutil file createnew genesis.blob 0
    fsutil file createnew aptos_root_key 0
    fsutil file createnew waypoint.txt 0
    Run docker-compose: docker-compose up
    ```

## Interacting with the local test validator network
After starting your local test validator network, you should see the following:

```
validator_1  | Entering test mode, this should never be used in production!
validator_1  | Completed generating configuration:
validator_1  | 	Log file: "/opt/aptos/var/validator.log"
validator_1  | 	Config path: "/opt/aptos/var/0/node.yaml"
validator_1  | 	Aptos root key path: "/opt/aptos/var/mint.key"
validator_1  | 	Waypoint: 0:7ff525d33f685a5cf26a71b393fa5159874c8f0c2861c382905f49dcb6991cb6
validator_1  | 	REST endpoint: 0.0.0.0:8080
validator_1  | 	FullNode network: /ip4/0.0.0.0/tcp/7180
validator_1  | 	ChainId: TESTING
```

This output contains information required for starting the Aptos CLI tool:
* Aptos root key path - The root (also known as a mint or faucet) key controls the account that can mint. Available in the docker compose folder under `aptos_root_key`.
* Waypoint - a verifiable checkpoint into the blockchain (available in the docker compose folder under waypoint.txt)
* REST endpoint - `http://127.0.0.1:8080`.
* ChainId - uniquely distinguishes this chain from other chains.

## Next Steps

At this point, you will have a special root account at `0x1` that can perform the mint operation. Follow up with 

* [Your first transaction](/tutorials/your-first-transaction) for how to submit transactions
* [Your first Move module](/tutorials/your-first-move-module) for how to create Move modules
* [Interacting with the Aptos Blockchain](/transactions/interacting-with-the-aptos-blockchain) to mint coins

It is important to note, that this solution does not include a Faucet. That is left as an exercise to the reader.
