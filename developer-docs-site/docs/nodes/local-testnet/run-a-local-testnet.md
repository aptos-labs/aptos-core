---
title: "Run a Local Testnet with Validator"
slug: "run-a-local-testnet"
sidebar_position: 9
---

# Run a Local Testnet with Validator

:::tip Using CLI to run a local testnet

If you want to use CLI to start and run a local testnet, see [Using CLI to Run a Local Testnet](./using-cli-to-run-a-local-testnet).
:::

You can run a local testnet of the Aptos blockchain. This local testnet will not be connected to the Aptos devnet. It will run on your local machine, independent of other Aptos networks. You can use this local testnet for testing and development purposes.

You can run a local testnet in two ways:

1. Using the Aptos-core source code. This approach is useful for testing modifications to the Aptos-core codebase or to the Aptos Framework.

2. Using Docker. This is particularly useful for building services on top of the Aptos blockchain or the Aptos Framework, as there is no build overhead and the ledger persists across network restarts (by default).

The rest of this document describes:

- How to start your local testnet with a single validator node, and
- How to start a Faucet service and attach it to your local testnet.

## Using the Aptos-core source code

1. Clone the Aptos repo.

    ```
    git clone https://github.com/aptos-labs/aptos-core.git
    ```

2. `cd` into `aptos-core` directory.

    ```
    cd aptos-core
    ```

3. Run the `scripts/dev_setup.sh` Bash script as shown below. This will prepare your developer environment.

    ```
    ./scripts/dev_setup.sh
    ```

4. Update your current shell environment.

    ```
    source ~/.cargo/env
    ```

5. With your development environment ready, now you can start your testnet network. Before you proceed, make a note of the following:

    :::tip
     - When you run the below command to start the local testnet, your terminal will enter into an interactive mode, with a message `Aptos is running, press ctrl-c to exit`. Hence, you will need to open another shell terminal for the subsequent steps described in this section.
     - After the below command runs, copy the `Test dir` information from the terminal output for the next step.
    :::

    To start your testnet locally, run the following command:

    ```
    CARGO_NET_GIT_FETCH_WITH_CLI=true cargo run -p aptos-node -- --test
    ```

    See below for an example of the partial output. Make a note of the `Test dir` from the output.

    ```
    ...
    ...
    ...

    Completed generating configuration:
        Log file: "/private/var/folders/gn/m74t8ylx55z935q8wx035qn80000gn/T/b3adc18c144bfcc78a1541953893bc1c/validator.log"
        Test dir: "/private/var/folders/gn/m74t8ylx55z935q8wx035qn80000gn/T/b3adc18c144bfcc78a1541953893bc1c/0/node.yaml"
        Aptos root key path: "/private/var/folders/gn/m74t8ylx55z935q8wx035qn80000gn/T/b3adc18c144bfcc78a1541953893bc1c/mint.key"
        Waypoint: 0:47e676b5fe38ebe2aec6053db7b3daa0b805693d6422e3475e46e89499464ecf
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        Fullnode network: /ip4/0.0.0.0/tcp/7180
    Aptos is running, press ctrl-c to exit
    ```

**NOTE**: The above command starts a local testnet with a single validator node. The command runs `aptos-node` from a genesis-only ledger state. If you want to reuse the ledger state produced by a previous run of `aptos-node`, then use:

```
cargo run -p aptos-node -- --test --config <config-path>
```

### Attaching a Faucet to your testnet

Faucets are stateless services that can be run in parallel with the testnet. A Faucet is a way to create Aptos test coins with no real-world value. You can use the Faucet by sending a request to create coins and transfer them into a given account on your behalf.

1. Make sure that you started your local testnet as described in Step 5 above.
2. Open a new shell terminal.
3. Copy the _Aptos root key path_ from your terminal where you started the testnet, and use it to replace the `mint-key-file-path` in the below command.
4. Run the following command to start a Faucet:
```
   cargo run --package aptos-faucet -- \
      --chain-id TESTING \
      --mint-key-file-path "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/mint.key" \
      --address 0.0.0.0 \
      --port 8000 \
      --server-url http://127.0.0.1:8080
```

This will start a Faucet running locally without any restrictions to tokens that can be claimed and minted. This Faucet service will be as accessible as the testnet you started above.

## Using Docker

This section describes how to start your local testing using Docker.

1. Install [Docker](https://docs.docker.com/get-docker/) including [Docker-Compose](https://docs.docker.com/compose/install/).
2. Create a directory for your local test validator network, and `cd` into it.
3. Download the YAML configuration files for:

  - [Validator testnet docker compose](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/validator-testnet/docker-compose.yaml) and
  - [Validator configuration](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/validator-testnet/validator_node_template.yaml).

4. Start Docker Compose by running the command:

    ```
    docker-compose up
    ```

### Example

An example command sequence for the above steps 2 through 4 is shown below:

```bash
mkdir aptos_local_validator && cd aptos_local_validator
wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/validator-testnet/docker-compose.yaml
wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/validator-testnet/validator_node_template.yaml
docker-compose up
```

This will start both a validator node and Faucet service.

- The Validator's REST endpoint will be available at `http://127.0.0.1:8080`, and
- The Faucet is available at `http://127.0.0.1:8000`.

### Troubleshooting

As the software is in the early stages of development, there may be breaking changes. If the software fails to start, do the following:

1. First, query Docker for both the containers and shared volumes with `docker container ls -a` and `docker volume ls`.
2. Then, delete them using `docker container rm $id` and `docker volume rm $name`.
3. Alternatively you can start with a clean slate by cleaning your entire local docker state by running the below command:

```bash
docker stop $(docker ps -a -q) && docker rm $(docker ps -a -q) && docker rmi $(docker images -q) && docker volume rm $(docker volume ls -q)
```
:::note

If you intend to use your testnet over an extended period of time, you should pin the images to a specific ID. Image IDs can be obtained via `docker container ls` and added to the docker compose file.

:::

## Interacting with the local test testnet

After starting your local testnet, you will see the following:

```
Entering test mode, this should never be used in production!
Completed generating configuration:
        Log file: "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/validator.log"
        Test dir: "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/0/node.yaml"
        Aptos root key path: "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/mint.key"
        Waypoint: 0:197bc8b76761622c2d2054d8bf93c1802fa0eb4bc55f0f3d4442878fdecc297f
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        Fullnode network: /ip4/0.0.0.0/tcp/7180

Aptos is running, press ctrl-c to exit
```

Use the [Aptos CLI tool](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md) to interact with your local testnet. The above output contains information you will use for starting the [Aptos CLI tool](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md):

* `Aptos root key path`: The root key (also known as the mint or faucet key) controls the account that can mint tokens. Available in the docker compose folder under `aptos_root_key`.
* `Waypoint`: A verifiable checkpoint of the blockchain (available in the docker compose folder under waypoint.txt)
* `REST endpoint`: The endpoint for the REST service, e.g., `http://127.0.0.1:8080`.
* `ChainId`: The chain ID uniquely distinguishes this network from other blockchain networks.

## Next steps

At this point, you will have a special root account at `0x1` that can perform the mint operation. Follow up with:

* [Your first transaction](/tutorials/your-first-transaction) to learn how to submit transactions.
* [Your first Move module](/tutorials/first-move-module) to learn how to create Move modules.
* [Interacting with the Aptos Blockchain](/guides/interacting-with-the-aptos-blockchain) to learn how to mint coins.
