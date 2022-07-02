---
title: "Using Docker"
slug: "run-validator-node-using-docker"
sidebar_position: 12
---

# Using Docker

1. Install Docker and Docker-Compose, [Aptos CLI 0.2.0](https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md).

:::caution Note on Apple M1

Docker has only been tested on Linux, Windows, and Intel macOS. If you are on M1 macOS, use the Aptos-core source approach.

:::

2. Create a directory for your Aptos node composition. e.g.
    ```
    export WORKSPACE=testnet
    mkdir ~/$WORKSPACE
    cd ~/$WORKSPACE
    ```

3. Download the validator.yaml and docker-compose.yaml configuration files into this directory.
    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/docker-compose.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/validator.yaml
    ```

4. Generate key pairs (node owner key, consensus key and networking key) in your working directory.

    ```
    aptos genesis generate-keys --output-dir ~/$WORKSPACE
    ```

    This will create three files: `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` for you. **IMPORTANT**: Backup your key files somewhere safe. These key files are important for you to establish ownership of your node, and you will use this information to claim your rewards later if eligible. Never share those keys with anyone else.

5. Configure validator information. You need to setup a static IP / DNS address which can be used by the node, and make sure the network / firewalls are properly configured to accept external connections. See [Network Identity For FullNode](../full-node/network-identity-fullnode.md) for how to do this.

    You will need this information to register on Aptos community website later.

    :::tip

    The `--full-node-host` flag is optional.

    :::

    ```
    cd ~/$WORKSPACE
    aptos genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username <select a username for your node> \
        --validator-host <Validator Node IP / DNS address>:<Port> \
        --full-node-host <Full Node IP / DNS address>:<Port>

    # for example, with IP:

    aptos genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username aptosbot \
        --validator-host 35.232.235.205:6180 \
        --full-node-host 34.135.169.144:6182

    # For example, with DNS:

    aptos genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username aptosbot \
        --validator-host bot.aptosdev.com:6180 \
        --full-node-host fn.bot.aptosdev.com:6182
    ```

    This will create a YAML file in your working directory with your username, e.g., `aptosbot.yaml`. It will look like below:

    ```
    ---
    account_address: 7410973313fd0b5c69560fd8cd9c4aaeef873f869d292d1bb94b1872e737d64f
    consensus_public_key: "0x4e6323a4692866d54316f3b08493f161746fda4daaacb6f0a04ec36b6160fdce"
    account_public_key: "0x83f090aee4525052f3b504805c2a0b1d37553d611129289ede2fc9ca5f6aed3c"
    validator_network_public_key: "0xa06381a17b090b8db5ffef97c6e861baad94a1b0e3210e6309de84c15337811d"
    validator_host:
      host: 35.232.235.205
      port: 6180
    full_node_network_public_key: "0xd66c403cae9f2939ade811e2f582ce8ad24122f0d961aa76be032ada68124f19"
    full_node_host:
      host: 35.232.235.206
      port: 6182
    stake_amount: 1
    ```

6. Create layout YAML file, which defines the node in the validatorSet, for test mode, we can create a genesis blob containing only one node.

    ```
    vi layout.yaml
    ```

    Add the public key for root account, node username, and chain_id in the `layout.yaml` file. For example:

    ```
    ---
    root_key: "F22409A93D1CD12D2FC92B5F8EB84CDCD24C348E32B3E7A720F3D2E288E63394"
    users:
      - "<username you specified from previous step>"
    chain_id: 40
    min_stake: 0
    max_stake: 100000
    min_lockup_duration_secs: 0
    max_lockup_duration_secs: 2592000
    epoch_duration_secs: 86400
    initial_lockup_timestamp: 1656615600
    min_price_per_gas_unit: 1
    allow_new_validators: true
    ```

    Please make sure you use the same root public key as shown in the example and same chain ID, those config will be used during registration to verify your node.

7. Download AptosFramework Move bytecode.

    Download the Aptos Framework from the release page: https://github.com/aptos-labs/aptos-core/releases/tag/aptos-framework-v0.2.0

    ```
    wget https://github.com/aptos-labs/aptos-core/releases/download/aptos-framework-v0.2.0/framework.zip
    unzip framework.zip
    ```

    You will now have a folder called `framework` in your ~/$WORKSPACE directory, and this folder contains Move bytecode files with format `.mv`.

8. Compile genesis blob and waypoint

    ```
    aptos genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ```

    This will create two files in your working directory, `genesis.blob` and `waypoint.txt`.

9. To recap, in your working directory, you should have a list of files:
    - `validator.yaml` validator config file
    - `docker-compose.yaml` docker compose file to run validator and fullnode
    - `private-keys.yaml` Private keys for owner account, consensus, networking
    - `validator-identity.yaml` Private keys for setting validator identity
    - `validator-full-node-identity.yaml` Private keys for setting validator full node identity
    - `<username>.yaml` Node info for both validator / fullnode
    - `layout.yaml` layout file to define root key, validator user, and chain ID
    - `framework` folder which contains all the move bytecode for AptosFramework.
    - `waypoint.txt` waypoint for genesis transaction
    - `genesis.blob` genesis binary contains all the info about framework, validatorSet and more.

10. Run docker-compose: `docker-compose up`. (or `docker compose up` depends on your version)

Now you have completed setting up your validator node in test mode. You can continue to our [Aptos community platform](https://community.aptoslabs.com/) website for registration. Additionally, you can also setup a fullnode following the instructions below.

11. [Optional] Now let's setup Fullnode on a different machine. Download the `fullnode.yaml` and `docker-compose-fullnode.yaml` configuration files into the working directory of Fullnode machine.
    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/docker-compose-fullnode.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/fullnode.yaml
    ```

12. Edit `fullnode.yaml` file to update the IP address for Validator node.

13. [Optional] Copy the `validator-full-node-identity.yaml`, `genesis.blob` and `waypoint.txt` files generated above into the same working directory on Fullnode machine.

14. [Optional] Run docker-compose: `docker-compose -f docker-compose-fullnode.yaml up`. Now you have successfully completed setting up your node in test mode. You can now proceed to the [Aptos community platform](https://community.aptoslabs.com/) website for registration.
