---
title: "Run a Local Testnet with Validator"
slug: "run-a-local-testnet"
sidebar_position: 9
---

# Run a Local Testnet with Validator

:::tip Using CLI to run a local testnet

If you want to use CLI to start and run a local testnet, see [Using CLI to Run a Local Testnet](./using-cli-to-run-a-local-testnet.md).
:::

You can run a local testnet of the Aptos blockchain. This local testnet will not be connected to the Aptos devnet. It will run on your local machine, independent of other Aptos networks. You can use this local testnet for testing and development purposes.

You can run a local testnet in two ways:

1. Using the Aptos-core source code. This approach is useful for testing modifications to the Aptos-core codebase or to the Aptos Framework.

2. Using Docker. This is particularly useful for building services on top of the Aptos blockchain or the Aptos Framework, as there is no build overhead and the ledger persists across network restarts (by default).

The rest of this document describes:

- How to start your local testnet with a single validator node, and
- How to start a Faucet service and attach it to your local testnet.

## Using the Aptos-core source code

1. Follow steps in [Building Aptos From Source](../../guides/building-from-source.md)

1. With your development environment ready, now you can start your testnet network. Before you proceed, make a note of the following:

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
cargo run --package aptos-faucet-service -- run-simple --key-file-path "/tmp/694173aa3bbe019499bbd5cf3fe0e2fc/mint.key" --node-url http://127.0.0.1:8080
```

This will start a Faucet running locally without any restrictions to tokens that can be claimed and minted. This Faucet service will be as accessible as the testnet you started above.

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

Use the [Aptos CLI tool](../../tools/aptos-cli/install-cli/index.md) to interact with your local testnet. The above output contains information you will use for starting the [Aptos CLI tool](../../tools/aptos-cli/use-cli/use-aptos-cli.md):

* `Aptos root key path`: The root key (also known as the mint or faucet key) controls the account that can mint tokens. Available in the docker compose folder under `aptos_root_key`.
* `Waypoint`: A verifiable checkpoint of the blockchain (available in the docker compose folder under waypoint.txt)
* `REST endpoint`: The endpoint for the REST service, e.g., `http://127.0.0.1:8080`.
* `ChainId`: The chain ID uniquely distinguishes this network from other blockchain networks.

## Next steps

At this point, you will have a special root account at `0x1` that can perform the mint operation. Follow up with:

* [Your first transaction](../../tutorials/first-transaction.md) to learn how to submit transactions.
* [Your first Move module](../../tutorials/first-move-module.md) to learn how to create Move modules.
* [Interacting with the Aptos Blockchain](../../tutorials/first-coin.md) to learn how to mint coins.
