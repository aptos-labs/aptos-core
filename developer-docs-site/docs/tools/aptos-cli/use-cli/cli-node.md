---
title: "Node"
id: "cli-node"
---

## Node command examples

This section summarizes how to run a local testnet with Aptos CLI. See [Run a Local Testnet with Aptos CLI](../../../nodes/local-testnet/using-cli-to-run-a-local-testnet.md) for more details.

For Aptos CLI commands applicable to validator nodes, see the [Owner](../../../nodes/validator-node/operator/staking-pool-operations.md#owner-operations-with-cli) and [Voter](../../../nodes/validator-node/voter/index.md#steps-using-aptos-cli) instructions.

### Running a local testnet

You can run a local testnet from the aptos CLI, that will match the version it was built with. Additionally, it can
run a faucet side by side with the local single node testnet.

```bash
$ aptos node run-local-testnet
Completed generating configuration:
        Log file: "/Users/greg/.aptos/testnet/validator.log"
        Test dir: "/Users/greg/.aptos/testnet"
        Aptos root key path: "/Users/greg/.aptos/testnet/mint.key"
        Waypoint: 0:d302c6b10e0fa68bfec9cdb383f24ef1189d8850d50b832365eea21ae52d8101
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        Fullnode network: /ip4/0.0.0.0/tcp/6181

Aptos is running, press ctrl-c to exit
```

This will have consistent state if the node is shutdown, it will start with the previous state.
If you want to restart the chain from genesis, you can add the `--force-restart` flag.

```bash
$ aptos node run-local-testnet --force-restart
Are you sure you want to delete the existing chain? [yes/no] >
yes
Completed generating configuration:
        Log file: "/Users/greg/.aptos/testnet/validator.log"
        Test dir: "/Users/greg/.aptos/testnet"
        Aptos root key path: "/Users/greg/.aptos/testnet/mint.key"
        Waypoint: 0:649efc34c813d0db8db6fa5b1ffc9cc62f726bb5168e7f4b8730bb155d6213ea
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        Fullnode network: /ip4/0.0.0.0/tcp/6181

Aptos is running, press ctrl-c to exit
```
