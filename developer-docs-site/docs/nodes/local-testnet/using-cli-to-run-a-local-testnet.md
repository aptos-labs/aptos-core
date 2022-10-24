---
title: "Using CLI to Run a Local Testnet"
id: "using-cli-to-run-a-local-testnet"
---

# Using CLI to Run a Local Testnet

:::tip Using source or Docker run a local testnet

If you want to use Docker or `aptos-core` source to start and run a local testnet, see [Run a Local Testnet with Validator](./run-a-local-testnet).
:::

You can run a local testnet of the Aptos blockchain. This local testnet will not be connected to the Aptos devnet. It will run on your local machine, independent of other Aptos networks. You can use this local testnet for testing and development purposes. A local testnet is a great tool for doing local development against a known version of the codebase without having to interact with a live network or deal with the real world costs of a live network.

:::tip Aptos CLI documentation
If you are new to Aptos CLI, then see this comprehensive [Aptos CLI documentation](/cli-tools/aptos-cli-tool/index.md).
:::

## Starting a local testnet with a faucet

You can start a local testnet using the following Aptos CLI command:

```bash
aptos node run-local-testnet --with-faucet
```

The above command will start a local validator node and will display a terminal output similar to the following:

```bash
Completed generating configuration:
        Log file: "/Users/greg/.aptos/testnet/validator.log"
        Test dir: "/Users/greg/.aptos/testnet"
        Aptos root key path: "/Users/greg/.aptos/testnet/mint.key"
        Waypoint: 0:74c9d14285ec19e6bd15fbe851007ea8b66efbd772f613c191aa78721cadac25
        ChainId: TESTING
        REST API endpoint: 0.0.0.0:8080
        Fullnode network: /ip4/0.0.0.0/tcp/6181

Aptos is running, press ctrl-c to exit

Faucet is running.  Faucet endpoint: 0.0.0.0:8081
```

The above command will use the default configuration for the validator node.

:::caution Do not use two instances of the same command at the same time
Note that two instances of the same command cannot run at the same time. This will result in a conflict on ports for the validator node.
:::

## Test with your local testnet

You can use the Aptos CLI for a full range of local testnet operations. See below for how to configure the CLI first.

### Configuring your Aptos CLI to use the local testnet

You can add a separate profile, as shown below:

```bash
aptos init --profile local --rest-url http://localhost:8080 --faucet-url http://localhost:8081
```

and you will get an output like below. At the `Enter your private key...` command prompt press enter to generate a random new key.

```bash
Configuring for profile local
Using command line argument for rest URL http://localhost:8080/
Using command line argument for faucet URL http://localhost:8081/
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]
```

This will create a new account and fund it with the default amount of coins, as shown below:

```bash
No key given, generating key...
Account 7100C5295ED4F9F39DCC28D309654E291845984518307D3E2FE00AEA5F8CACC1 doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account 7100C5295ED4F9F39DCC28D309654E291845984518307D3E2FE00AEA5F8CACC1!  Run `aptos help` for more information about commands
{
  "Result": "Success"
}
```

From now on you should add `--profile local` to the commands to run them on the local testnet.

## Creating and funding accounts

To create new accounts on the local testnet, we recommend using the above instructions with different profile names:

```bash
PROFILE=local
aptos init --profile $PROFILE --rest-url http://localhost:8080 --faucet-url http://localhost:8081
```

To fund accounts:

```bash
aptos account fund --profile $PROFILE --account $PROFILE
```

To create resource accounts:

```bash
aptos account create-resource-account --profile $PROFILE --seed 1
```

## Publishing modules to the local testnet

You can run any command by adding the `--profile $PROFILE` flag.  In this case, we also use `$PROFILE` as the named address in the `HelloBlockchain` example.

```bash
aptos move publish --profile $PROFILE --package-dir /opt/git/aptos-core/aptos-move/move-examples/hello_blockchain --named-addresses HelloBlockchain=$PROFILE
{
  "Result": {
    "changes": [
      {
        "address": "7100c5295ed4f9f39dcc28d309654e291845984518307d3e2fe00aea5f8cacc1",
        "data": {
          "authentication_key": "0x7100c5295ed4f9f39dcc28d309654e291845984518307d3e2fe00aea5f8cacc1",
          "coin_register_events": {
            "counter": "1",
            "guid": {
              "id": {
                "addr": "0x7100c5295ed4f9f39dcc28d309654e291845984518307d3e2fe00aea5f8cacc1",
                "creation_num": "0"
              }
            }
          },
          "sequence_number": "4"
        },
        "event": "write_resource",
        "resource": "0x1::account::Account"
      },
    ...
    ],
    "gas_used": 59,
    "success": true,
    "version": 6261,
    "vm_status": "Executed successfully"
  }
}
```

## Resetting the local state

If you updated your codebase with backwards incompatible changes, or just want to start over, you can run
the command with the `--force-restart` flag:

```bash
aptos node run-local-testnet --with-faucet --force-restart
```

It will then prompt you if you really want to restart the chain, to ensure that you do not delete your work by accident.

```bash
Are you sure you want to delete the existing chain? [yes/no] >
```

## FAQ

### I'm getting the error `address already in use`, what can I do?

If you're getting an error similar to this error:

```bash
'panicked at 'error binding to 0.0.0.0:9101: error creating server listener: Address already in use (os error 48)'
```

This means you are either already running a node, or you have another process running on that port.

On macOS and Linux, you can run the following command to get the name and PID of the process using the port:

```bash
PORT=9101
lsof -i :$PORT
```

### Where can I get more information about the run-local-testnet command?

More CLI help can be found by running the command:

```bash
aptos node run-local-testnet --help
```

which will provide information about each of the flags for the command.
