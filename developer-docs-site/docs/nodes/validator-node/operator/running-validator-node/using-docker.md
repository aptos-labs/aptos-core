---
title: "Using Docker"
slug: "run-validator-node-using-docker"
---

# Using Docker

This is a step-by-step guide to install an Aptos node using Docker. Follow these steps to configure a validator node and a validator fullnode on separate machines. Use the `fullnode.yaml` to run a validator fullnode. See [Step 11](#docker-vfn).

## Before you proceed

Make sure the following are installed on your local computer:
   - **Aptos CLI**: https://aptos.dev/tools/aptos-cli/install-cli/index
   - **Docker and Docker-compose:** https://docs.docker.com/engine/install/

:::caution Note on Apple M1

Docker method has only been tested on Linux, Windows, and Intel macOS. If you are on M1 macOS, use the [Aptos-core source approach](./using-source-code.md).

:::

1. Create a directory for your Aptos node composition, and pick a username for your node. e.g.
    ```bash
    export WORKSPACE=mainnet
    export USERNAME=alice
    mkdir ~/$WORKSPACE
    cd ~/$WORKSPACE
    ```

2. Download the following files by following the download commands on the [Node Files](../../../node-files-all-networks/node-files.md) page:
    - `validator.yaml`
    - `docker-compose.yaml`
    - `docker-compose-fullnode.yaml`
    - `haproxy.cfg`
    - `haproxy-fullnode.cfg`
    - `blocked.ips`

3. Generate the key pairs (node owner, voter, operator key, consensus key and networking key) in your working directory.

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

4. Configure validator information. You need to setup a static IP / DNS address (DNS is much preferred) which can be used by the node, and make sure the network / firewalls are properly configured to accept external connections. See [Network Identity For Fullnode](../../../full-node/network-identity-fullnode.md) for how to do this. 

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

5. Download the following files by following the download commands on the [Node Files](../../../node-files-all-networks/node-files.md) page:
    - `genesis.blob`
    - `waypoint.txt`

6. <span id="docker-files">To recap, in your working directory, you should have a list of files:</span>

    - `docker-compose.yaml` docker compose file to run validator and fullnode
    - `keys` folder containing:
      - `public-keys.yaml`: Public keys for the owner account, consensus, networking (from step 4).
      - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 4).
      - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 4).
      - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 4).
    - `username` folder containing: 
      - `owner.yaml`: define owner, operator, and voter mapping. They are all the same account in test mode (from step 5).
      - `operator.yaml`: Node information that will be used for both the Validator and the fullnode (from step 5). 
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 6).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validatorSet and more (from step 6).

7. Run docker-compose: `docker-compose up`. (or `docker compose up` depends on your version)

**Now you have completed setting up your validator node. Next, setup a validator fullnode following the instructions below.**

9. <span id="docker-vfn">Set up a validator fullnode on a different machine. Download the `fullnode.yaml` and `docker-compose-fullnode.yaml` configuration files into the working directory of fullnode machine.</span> See [Node Files](../../../node-files-all-networks/node-files.md) for a full list of files you should download and the download commands. 

10.  Edit `fullnode.yaml` file to update the IP address for validator node.

11.  Copy the `validator-full-node-identity.yaml`, download `genesis.blob` and `waypoint.txt` files into the same working directory on fullnode machine.

12.  Run docker-compose: `docker-compose -f docker-compose-fullnode.yaml up`.

Now you have successfully completed setting up your node.

Now proceed to [connecting to the Aptos network](../connect-to-aptos-network.md) and [establishing staking pool operations](../staking-pool-operations.md).

<!-- 1.  Optional: if you need to block an ip address simply add it to the bottom of blocked.ips and reload haproxy -->
