---
title: "Using Source Code"
slug: "run-validator-node-using-source"
---

# Using Aptos-core source code

This is a step-by-step guide to install an Aptos node using source code. Follow these steps to configure a validator node and a validator fullnode on separate machines. Use the `fullnode.yaml` to run a validator fullnode&mdash;see Step 12.

## Before you proceed

Make sure the following are installed on your local computer:
   - **Aptos CLI**: https://aptos.dev/tools/aptos-cli/install-cli/index

## Install

:::tip One validator node + one validator fullnode
Follow the below instructions **twice**, i.e., first on one machine to run a validator node and the second time on another machine to run a validator fullnode. 
:::

1. Follow steps in [Building Aptos From Source](../../../../guides/building-from-source.md)

5. Checkout the `mainnet` branch using `git checkout --track origin/mainnet`.

6. Create a directory for your Aptos node composition, and pick a username for your node. e.g.
    ```bash
    export WORKSPACE=mainnet
    export USERNAME=alice
    mkdir ~/$WORKSPACE
    ```

7. Generate the key pairs (node owner, voter, operator key, consensus key and networking key) in your working directory.

    ```bash
    aptos genesis generate-keys --output-dir ~/$WORKSPACE/keys
    ```

    This will create 4 key files under `~/$WORKSPACE/keys` directory: 
      - `public-keys.yaml`
      - `private-keys.yaml`
      - `validator-identity.yaml`, and
      - `validator-full-node-identity.yaml`.
      
      :::danger IMPORTANT

       Backup your `private-keys.yaml` somewhere safe. These keys are important for you to establish ownership of your node. **Never share private keys with anyone.**
      :::

8. Configure validator information. You need to setup a static IP / DNS address (DNS is much preferred) which can be used by the node, and make sure the network / firewalls are properly configured to accept external connections.

    ```bash
    cd ~/$WORKSPACE
    aptos genesis set-validator-configuration \
        --local-repository-dir ~/$WORKSPACE \
        --username $USERNAME \
        --owner-public-identity-file ~/$WORKSPACE/keys/public-keys.yaml \
        --validator-host <validator node IP / DNS address>:<Port> \
        --full-node-host <Full Node IP / DNS address>:<Port> \
        --stake-amount 100000000000000

    # for example, with IP:

    aptos genesis set-validator-configuration \
        --local-repository-dir ~/$WORKSPACE \
        --username $USERNAME \
        --owner-public-identity-file ~/$WORKSPACE/keys/public-keys.yaml \
        --validator-host 35.232.235.205:6180 \
        --full-node-host 34.135.169.144:6182 \
        --stake-amount 100000000000000

    # For example, with DNS:

    aptos genesis set-validator-configuration \
        --local-repository-dir ~/$WORKSPACE \
        --username $USERNAME \
        --owner-public-identity-file ~/$WORKSPACE/keys/public-keys.yaml \
        --validator-host bot.aptosdev.com:6180 \
        --full-node-host fn.bot.aptosdev.com:6182 \
        --stake-amount 100000000000000
    ```

    This will create two YAML files in the `~/$WORKSPACE/$USERNAME` directory: `owner.yaml` and `operator.yaml`. 

9. Download the following files by following the download commands on the [Node Files](../../../node-files-all-networks/node-files.md) page: 
   - `validator.yaml`
   - `fullnode.yaml`
   - `genesis.blob`
   - `waypoint.txt`
   - `haproxy.cfg`
   - `haproxy-fullnode.cfg` and
   - `blocked.ips`
   - `docker-compose-src.yaml`

10. Copy the `validator.yaml`, `fullnode.yaml` files into ~/$WORKSPACE/config/ directory.
    ```bash
    mkdir ~/$WORKSPACE/config
    cp validator.yaml ~/$WORKSPACE/config/validator.yaml
    cp fullnode.yaml ~/$WORKSPACE/config/fullnode.yaml
    ```

    Modify the config files to update the data directory, key path, genesis file path, waypoint path. User must have write access to data directory.

11. <span id="source-code-vfn">To recap, in your working directory (`~/$WORKSPACE`), you should have a list of files:</span>

    - `config` folder containing:
      - `validator.yaml` validator config file
      - `fullnode.yaml` fullnode config file
    - `keys` folder containing:
      - `public-keys.yaml`: Public keys for the owner account, consensus, networking (from step 7).
      - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 7).
      - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 7).
      - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 7).
    - `username` folder containing: 
      - `owner.yaml`: Define owner, operator, and voter mapping. They are all the same account in test mode (from step 8).
      - `operator.yaml`: Node information that will be used for both the Validator and the fullnode (from step 8). 
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 9).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validatorSet and more (from step 9).

12. Start your validator by running the below commands, with the paths assuming you are in the root of the `aptos-core` directory:

    ```bash
    cargo clean
    cargo build -p aptos-node --release
    sudo mv target/release/aptos-node /usr/local/bin
    aptos-node -f ~/$WORKSPACE/config/validator.yaml
    ```

    Run validator fullnode on **another machine**:

    ```bash
    cargo clean
    cargo build -p aptos-node --release
    sudo mv target/release/aptos-node /usr/local/bin
    aptos-node -f ~/$WORKSPACE/config/fullnode.yaml
    ```

Optionally, you may set up `aptos-node` to run as a service controlled by `systemctl` in a file resembling:

```bash
[Unit]
Description=Aptos Node Service

[Service]
User=nodeuser
Group=nodeuser

LimitNOFILE=500000

#Environment="RUST_LOG=error"
WorkingDirectory=/home/nodeuser/aptos-core
ExecStart=/usr/local/bin/aptos-node -f /home/nodeuser/aptos-mainnet/config/validator.yaml

Restart=on-failure
RestartSec=3s

StandardOutput=journal
StandardError=journal
SyslogIdentifier=aptos-node

[Install]
WantedBy=multi-user.target
```

You have completed setting up your node.

Now proceed to [connecting to the Aptos network](../connect-to-aptos-network.md) and [establishing staking pool operations](../staking-pool-operations.md).