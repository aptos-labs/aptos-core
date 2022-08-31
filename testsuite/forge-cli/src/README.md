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
2022-09-01T15:41:28.090606Z [main] INFO testsuite/forge/src/backend/local/swarm.rs:207 The root (or mint) key for the swarm is: 0x4A7ED...
...
2022-09-01T15:41:28.094800Z [main] INFO testsuite/forge/src/backend/local/node.rs:129 Started node "0" (PID: 78939) with command: ".../aptos-core/target/debug/aptos-node" "-f" "/private/var/folders/dx/c0l2rrkn0656gfx6v5_dy_p80000gn/T/.tmpq9uPMJ/0/node.yaml"
2022-09-01T15:41:28.094825Z [main] INFO testsuite/forge/src/backend/local/node.rs:137 Node "0" REST API is listening at: 127.0.0.1:50271/v1
2022-09-01T15:41:28.094838Z [main] INFO testsuite/forge/src/backend/local/node.rs:142 Node "0" Inspection service is listening at 127.0.0.1:50273
...
```

Using the information from this output, you could stop a single node and restart
it, e.g., stop and restart node `0`:

```
kill -9 78939
cargo run -p aptos-node -- -f "/private/var/folders/dx/c0l2rrkn0656gfx6v5_dy_p80000gn/T/.tmpq9uPMJ/0/node.yaml"
```

## Faucet and Minting

In addition, the swarm output also displays the root (or mint) key for the network. This allows
you to run a local faucet and start minting test tokens in the network. For this, simply run the
faucet command using the mint key and point it to the REST API of one of the nodes, e.g. node `0`:

```
cargo run --bin aptos-faucet -- -c TESTING --mint-key 0x4A7ED... -s http://127.0.0.1:50271 -p 8081   
```

The above command will run a faucet locally, listening on port `8081`. Using this faucet, you could
then mint tokens to your test accounts, e.g.,:

```
curl -X POST http://127.0.0.1:8081/mint\?amount\=1000000\&pub_key\=459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d\&return_txns\=true
01000000000000000000000000000000dd05a600000000000001e001a11ceb0b010000000701000202020403061004160205181d0735600895011000000001010000020001000003020301010004010300010501060c0108000506080005030a020a020005060c05030a020a020109000b4469656d4163636f756e741257697468647261774361706162696c6974791b657874726163745f77697468647261775f6361706162696c697479087061795f66726f6d1b726573746f72655f77697468647261775f6361706162696c69747900000000000000000000000000000001010104010c0b0011000c050e050a010a020b030b0438000b051102020107000000000000000000000000000000010358555303585553000403a74fd7c46952c497e75afb0a7932586d0140420f00000000000400040040420f00000000000000000000000000035855532a610f6000000000020020056244e7bf776e471d818dc18fdf7b8833c5439ac9a96e126f8f32c7bc7c14b64026a2c45c8e4066c661dc4f36baa6ad61499999b548b9f63ad15853660c408cedec3078b7773a829ec48de8b04291cd11530734b2f91d5e42f35a4c6378cb7c09
```

See more about how the faucet works in the [README](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos-faucet).

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
