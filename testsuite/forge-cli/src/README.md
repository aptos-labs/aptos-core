---
id: forge cli
title: Forge CLI
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/testsuite/forge-cli/README.md
---

# Forge CLI

This crate contains the Forge command line interface (CLI) tool. This enables users to
run local and remote Aptos swarms (i.e., networks of validators and validator fullnodes). For
example, to deploy a local validator swarm, run:

```
cargo run -p forge-cli -- --suite "run_forever" --num-validators 4 test local-swarm
```

This will start a local network of 4 validators, each running in their own process. The
network will run forever, unless manually killed. The output will display the locations
of the validator files (e.g., the genesis files, logs, node configurations, etc.) and the
commands that were run to start each node. The process id (PID) of each node and
server addresses (e.g., REST APIs) are also displayed when it starts. For example, if you
run the above command you should see:

```
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

Using the information from this output, you could stop a single node and restart
it, e.g., stop and restart node `0`:

```
kill -9 <Node 0 PID>
cargo run -p aptos-node -- -f <Location to the node 0 configuration file displayed above>
```

## Faucet and Minting

In addition, the swarm output also displays the root (or mint) key for the network. This allows
you to run a local faucet and start minting test tokens in the network. For this, simply run the
faucet command using the mint key and point it to the REST API of one of the nodes, e.g. node `0`:

```
cargo run --bin aptos-faucet -- -c TESTING --mint-key <Root/mint key displayed above> -s <URL for node 0 REST API> -p 8081   
```

The above command will run a faucet locally, listening on port `8081`. Using this faucet, you could
then mint tokens to your test accounts, e.g.,:

```
curl -X POST http://127.0.0.1:8081/mint\?amount\=<amount to mint>\&pub_key\=<public key to mint tokens to>\&return_txns\=true
01000000000000000000000000000000dd05a600000000000001e001a11ceb0b01000...
```

See more about how the faucet works in the [README](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos-faucet).
Likewise, see the documentation about how to use the [Aptos CLI](https://aptos.dev/cli-tools/aptos-cli-tool/use-aptos-cli) with an existing faucet.

## Validator fullnodes

To also run validator fullnodes inside the network, use the `--num-validator-fullnodes` flag, e.g.,:
```
cargo run -p forge-cli -- --suite "run_forever" --num-validators 3 --num-validator-fullnodes 1 test local-swarm
```

## Additional usage

To see all tool usage options, run:
```
cargo run -p forge-cli --help
```
