---
title: "Running a Local Multi-node Network"
slug: "running-a-local-multi-node-network"
---

# Running a Local Multi-node Network

This guide describes how to run a local network with multiple validator nodes and validator fullnodes. You will use the [Aptos Forge CLI](https://github.com/aptos-labs/aptos-core/tree/main/testsuite/forge-cli/src) for this.

:::tip Use only for test deployments
The method described in this guide should be used only for test deployments of multi-node local networks. Do not use this guide for deploying in production environments. Currently this is the only guide for multi-node deployments. 

For deploying a local network with a single node, see [Running Local Testnet](/nodes/local-testnet/index.md) and [Local testnet development flow](/guides/local-testnet-dev-flow.md).
:::

## Before you proceed

Make sure you cloned the Aptos source GitHub repo by following these steps:

1. Clone the Aptos repo.

```bash
git clone https://github.com/aptos-labs/aptos-core.git
```

2. `cd` into aptos-core directory.

```bash
cd aptos-core
```

3. Run the scripts/dev_setup.sh Bash script as shown below. This will prepare your developer environment.

```bash
./scripts/dev_setup.sh
```

3. Update your current shell environment.

```bash
source ~/.cargo/env
```

With your development environment ready, now you can proceed below.

## Running multiple validators

To deploy multiple local validators, run:

```bash
cargo run -p forge-cli \
        -- \
        --suite "run_forever" \
        --num-validators 4 test local-swarm
```

This will start a local network of 4 validators, each running in their own process. The network will run forever unless you manually terminate it. 

The terminal output will display the locations of the validator files (for example, the genesis files, logs, node configurations, etc.) and the commands that were run to start each node. The process id (PID) of each node and server addresses (e.g., REST APIs) are also displayed when it starts. For example, if you run the above command you should see:

```bash
...
2022-09-01T15:41:27.228289Z [main] INFO crates/aptos-genesis/src/builder.rs:462 Building genesis with 4 validators. Directory of output: "/private/var/folders/dx/c0l2rrkn0656gfx6v5_dy_p80000gn/T/.tmpq9uPMJ"
...
2022-09-01T15:41:28.090606Z [main] INFO testsuite/forge/src/backend/local/swarm.rs:207 The root (or mint) key for the swarm is: 0xf9f...
...
2022-09-01T15:41:28.094800Z [main] INFO testsuite/forge/src/backend/local/node.rs:129 Started node 0 (PID: 78939) with command: ".../aptos-core/target/debug/aptos-node" "-f" "/private/var/folders/dx/c0l2rrkn0656gfx6v5_dy_p80000gn/T/.tmpq9uPMJ/0/node.yaml"
2022-09-01T15:41:28.094825Z [main] INFO testsuite/forge/src/backend/local/node.rs:137 Node 0: REST API is listening at: http://127.0.0.1:64566
2022-09-01T15:41:28.094838Z [main] INFO testsuite/forge/src/backend/local/node.rs:142 Node 0: Inspection service is listening at http://127.0.0.1:64568
...
```

Using the information from this output, you can stop a single node and restart
it. For example, to stop and restart the node `0`, execute the below commands:

```bash
kill -9 <Node 0 PID>
cargo run -p aptos-node \
        -- \
        -f <Location to the node 0 configuration file displayed above>
```

## Faucet and minting

In addition, the terminal output also displays the root (or mint) key for the network. This allows you to run a local faucet and start minting test tokens in the network. For this, simply run the faucet command using the mint key and point it to the REST API of one of the nodes, for example, node `0`:

```bash
cargo run --bin aptos-faucet \
        -- \
        -c TESTING \
        --mint-key <Root/mint key displayed above> \
        -s <URL for node 0 REST API> \
        -p 8081   
```

The above command will run a faucet locally, listening on port `8081`. Using this faucet, you could then mint tokens to your test accounts, for example:

```bash
curl -X POST http://127.0.0.1:8081/mint\?amount\=<amount to mint>\&pub_key\=<public key to mint tokens to>\&return_txns\=true
01000000000000000000000000000000dd05a600000000000001e001a11ceb0b01000...
```

:::tip Faucet and Aptos CLI
See more on how the faucet works in the [README](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos-faucet).

Also see how to use the [Aptos CLI](https://aptos.dev/cli-tools/aptos-cli-tool/use-aptos-cli/#account-examples) with an existing faucet.
:::

## Validator fullnodes

To also run validator fullnodes inside the network, use the `--num-validator-fullnodes` flag. For example:
```bash
cargo run -p forge-cli \
        -- \
        --suite "run_forever" \
        --num-validators 3 \
        --num-validator-fullnodes 1 test local-swarm
```

## Additional usage

To see all tool usage options, run:
```bash
cargo run -p forge-cli --help
```