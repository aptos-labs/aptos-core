---
title: "Run Validator Node Using Source Code"
slug: "run-validator-node-using-source"
sidebar_position: 13
---

# Using Aptos-core source code

1. Clone the Aptos repo.

      ```
      git clone https://github.com/<YOUR-GITHUB-USERID>/aptos-core

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

With your development environment ready, now you can start to setup your Validator node.

5. Checkout the `testnet` branch using `git checkout --track origin/testnet`.

6. Create a directory for your Aptos node composition. e.g.
    ```
    export WORKSPACE=testnet
    mkdir ~/$WORKSPACE
    ```

7. Generate key pairs (node owner key, consensus key and networking key) in your working directory.

    ```
    cargo run --release -p aptos -- genesis generate-keys --output-dir ~/$WORKSPACE
    ```

    This will create three files: `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` for you. **IMPORTANT**: Backup your key files somewhere safe. These key files are important for you to establish ownership of your node, and you will use this information to claim your rewards later if eligible. Never share those keys with anyone else.

8. Configure validator information, you need to setup a static IP / DNS address which can be used by the node, and make sure the network / firewalls are properly configured to accept external connections. This is all the info you need to register on our community website later.

    ```
    cargo run --release -p aptos -- genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username <pick a username for your node> \
        --validator-host <Validator Node IP / DNS address>:<Port> \
        --full-node-host <Full Node IP / DNS address>:<Port>

    # for example, with IP:

    cargo run --release -p aptos -- genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username aptosbot \
        --validator-host 35.232.235.205:6180 \
        --full-node-host 34.135.169.144:6182

    # for example, with DNS:

    cargo run --release -p aptos -- genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username aptosbot \
        --validator-host bot.aptosdev.com:6180 \
        --full-node-host fn.bot.aptosdev.com:6182
    ```

    This will create a YAML file in your working directory with your username, e.g. `aptosbot.yml`, it should looks like:

    ```
    ---
    account_address: 7410973313fd0b5c69560fd8cd9c4aaeef873f869d292d1bb94b1872e737d64f
    consensus_key: "0x4e6323a4692866d54316f3b08493f161746fda4daaacb6f0a04ec36b6160fdce"
    account_key: "0x83f090aee4525052f3b504805c2a0b1d37553d611129289ede2fc9ca5f6aed3c"
    network_key: "0xa06381a17b090b8db5ffef97c6e861baad94a1b0e3210e6309de84c15337811d"
    validator_host:
      host: 35.232.235.205
      port: 6180
    full_node_host:
      host: 34.135.169.144
      port: 6182
    stake_amount: 1
    ```

9. Create layout YAML file, which defines the node in the validatorSet, for test mode, we can create a genesis blob containing only one node.

    ```
    vi ~/$WORKSPACE/layout.yaml
    ```

    Add the public key for root account, node username, and chain_id in the `layout.yaml` file, for example:

    ```
    ---
    root_key: "0x5243ca72b0766d9e9cbf2debf6153443b01a1e0e6d086c7ea206eaf6f8043956"
    users:
      - <username you created in step 8>
    chain_id: 23
    ```

    You can use the same root key as the example, or generate new one yourself by running `cargo run -p aptos -- key generate --output-file <file name>`

10. Build AptosFramework Move bytecodes, copy into the framework folder

    ```
    cargo run --release --package framework -- --package aptos-framework --output current

    mkdir ~/WORKSPACE/framework

    mv aptos-framework/releases/artifacts/current/build/**/bytecode_modules/*.mv ~/$WORKSPACE/framework/
    ```

    You should now have a folder called `framework`, which contains move bytecodes with format `.mv`.

11. Compile genesis blob and waypoint

    ```
    cargo run --release -p aptos -- genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ```

    This should create two files in your working directory, `genesis.blob` and `waypoint.txt`

12. Copy the `validator.yaml`, `fullnode.yaml` files into this directory.
    ```
    mkdir ~/$WORKSPACE/config
    cp docker/compose/aptos-node/validator.yaml ~/$WORKSPACE/validator.yaml
    cp docker/compose/aptos-node/fullnode.yaml ~/$WORKSPACE/fullnode.yaml
    ```

    Modify the config file to update the key path, genesis file path, waypoint path.
    ```
    QUOTED_WORKSPACE=$(echo $HOME/$WORKSPACE | sed -e 's/\//\\\//g')
    sed -i -e "s/\/opt\/aptos\/genesis/$QUOTED_WORKSPACE/g" ~/$WORKSPACE/validator.yaml
    sed -i -e "s/\/opt\/aptos\/genesis/$QUOTED_WORKSPACE/g" ~/$WORKSPACE/fullnode.yaml
    ```

    There are some changes to be done about network configuration.
    The top level overview of what we need to achieve is the following: there are two
    types of networks, full-node-network and validator-network. Each of network
    can also have some subnetworks. We need to help our nodes to find each other
    in some subset of networks.

    So, let's remember the step 8, where we were specifying 
    `--validator-host <Validator Node IP / DNS address>:<Port>`.
    This should be an address validator listening to on a validator_network.
    It is [here](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/aptos-node/validator.yaml#L26).
    Port 6180 is a default port to listen, so it is not specified in a config. But again,
    you have to make this line consistent with a port you specified on validator
    in step 8. If in step 8 you left the value for port as 6180, no edits are required
    for `validator.yaml`. Second thing is a port for full_node_network for our validator.
    By default it [listens](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/aptos-node/validator.yaml#L35)
    on port 6181. It is absolutely fine if you changed it, just keep this number in mind.
    We will refer 6180 as VALIDATOR_LISTENING_VALIDATOR_NETWORK and 6181 as
    VALIDATOR_LISTENING_FULL_NETWORK later.

    Now, it is time to update `fullnode.yaml`. It has two full_node_networks.
    First one is [private](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/aptos-node/fullnode.yaml#L11).
    If you are running validator and full node on a same machine, it makes sense to change the
    port for private network to anything. It does not matter. For example 16181.
    The important thing here is to specify the address of our validator.
    So in the [L17](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/aptos-node/fullnode.yaml#L17)
    you need to specify a correct IP address where you validator is located (or 127.0.0.1 if it is on a same host),
    and VALIDATOR_LISTENING_FULL_NETWORK port.

    [Line 22](https://github.com/aptos-labs/aptos-core/blob/main/docker/compose/aptos-node/fullnode.yaml#L22)
    defines FULL_NODE_LISTENING_PUBLIC_NETWORK port, in this case 6182.

    In the submission form you need to specify VALIDATOR_LISTENING_VALIDATOR_NETWORK and
    FULL_NODE_LISTENING_PUBLIC_NETWORK ports.

:::note

If you have an intention to run both validator and full nodes on a same server,
some extra changes to config are required. The instruction for doing this is not
a part of this documentation, as it requires some understanding of implications
of that actions.

Broadly speaking, these updates in either validator.yaml or fullnode.yaml are required:

a) use different `base.data_dir` values
b) use different `full_node_networks.listen_address` values
c) use different `api.address` values
d) append the following sections to one of the configs (you can change port values to some
   other reasonable values)
```
debug\_interface:
  admission\_control\_node\_debug\_port: 16192
  public\_metrics\_server\_port: 19103
  metrics\_server\_port: 19104

storage:
  address: 127.0.0.1:16666
  backup\_service\_address: 127.0.0.1:16186
```

:::


13. To recap, in your working directory (`~/$WORKSPACE`), you should have a list of files:
    - `validator.yaml` validator config file
    - `fullnode.yaml` fullnode config file
    - `private-keys.yaml` Private keys for owner account, consensus, networking
    - `validator-identity.yaml` Private keys for setting validator identity
    - `validator-full-node-identity.yaml` Private keys for setting validator full node identity
    - `<username>.yaml` Node info for both validator / fullnode
    - `layout.yaml` layout file to define root key, validator user, and chain ID
    - `framework` folder which contains all the move bytecode for AptosFramework.
    - `waypoint.txt` waypoint for genesis transaction
    - `genesis.blob` genesis binary contains all the info about framework, validatorSet and more.

14. Start your local Validator by running the below command:

    ```
    cargo run -p aptos-node --release -- -f ~/$WORKSPACE/validator.yaml
    ```

    Run fullnode in another terminal:

    ```
    cargo run -p aptos-node --release -- -f ~/$WORKSPACE/fullnode.yaml
    ```

Now you have completed setting up your node in test mode. You can continue to our [community](https://community.aptoslabs.com/) website for registration.
