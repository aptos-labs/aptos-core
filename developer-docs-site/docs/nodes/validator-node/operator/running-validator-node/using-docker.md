---
title: "Using Docker"
slug: "run-validator-node-using-docker"
---

# Using Docker

:::tip For validator fullnode
Use the `fullnode.yaml` to run a validator fullnode. See [Step 11](#docker-vfn).
:::

1. Install Docker and Docker-Compose, [Aptos CLI 0.3.1](https://aptos.dev/cli-tools/aptos-cli-tool/install-aptos-cli).

:::caution Note on Apple M1

Docker has only been tested on Linux, Windows, and Intel macOS. If you are on M1 macOS, use the Aptos-core source approach.

:::

2. Create a directory for your Aptos node composition, and pick a username for your node. e.g.
    ```
    export WORKSPACE=testnet
    export USERNAME=alice
    mkdir ~/$WORKSPACE
    cd ~/$WORKSPACE
    ```

3. Download the validator.yaml, docker-compose.yaml, haproxy.cfg, and blocked.ips configuration files into this directory.
    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/docker-compose.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/validator.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/haproxy.cfg
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/blocked.ips
    ```

4. Generate the key pairs (node owner, voter, operator key, consensus key and networking key) in your working directory.

    ```
    aptos genesis generate-keys --output-dir ~/$WORKSPACE/keys
    ```

    This will create 4 key files under `~/$WORKSPACE/keys` directory: 
      - `public-keys.yaml`
      - `private-keys.yaml`
      - `validator-identity.yaml`, and
      - `validator-full-node-identity.yaml`.
      
      :::caution IMPORTANT

       Backup your private key files somewhere safe. These key files are important for you to establish ownership of your node. **Never share private keys with anyone.**
      :::

5. Configure validator information. You need to setup a static IP / DNS address (DNS is much preferred) which can be used by the node, and make sure the network / firewalls are properly configured to accept external connections. See [Network Identity For Fullnode](/docs/nodes/full-node/network-identity-fullnode.md) for how to do this. 

    You will need this information to register on Aptos community website later.

    :::tip

    The `--full-node-host` flag is optional.

    :::

    ```
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

6. Download the genesis blob and waypoint for the network you want to connect to, you can find a full list of networks [here](https://github.com/aptos-labs/aptos-genesis-waypoint)

  For example, to download testnet genesis and waypoint:

  ```
  curl https://raw.githubusercontent.com/aptos-labs/aptos-genesis-waypoint/main/testnet/waypoint.txt -o waypoint.txt
  curl https://raw.githubusercontent.com/aptos-labs/aptos-genesis-waypoint/main/testnet/genesis.blob -o genesis.blob
  ```

7. <span id="docker-files">To recap, in your working directory, you should have a list of files:</span>

    - `docker-compose.yaml` docker compose file to run validator and fullnode
    - `keys` folder, which includes:
      - `public-keys.yaml`: Public keys for the owner account, consensus, networking (from step 4).
      - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 4).
      - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 4).
      - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 4).
    - `username` folder, which includes: 
      - `owner.yaml`: define owner, operator, and voter mapping. They are all the same account in test mode (from step 5).
      - `operator.yaml`: Node information that will be used for both the Validator and the fullnode (from step 5). 
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 6).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validatorSet and more (from step 6).

8. Run docker-compose: `docker-compose up`. (or `docker compose up` depends on your version)

Now you have completed setting up your validator node. Additionally, you can also setup a validator fullnode following the instructions below.

9. <span id="docker-vfn">Now let's setup fullnode on a different machine. Download the `fullnode.yaml` and `docker-compose-fullnode.yaml` configuration files into the working directory of fullnode machine.</span>

    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/docker-compose-fullnode.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/fullnode.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/haproxy.cfg
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/blocked.ips
    ```

10.  Edit `fullnode.yaml` file to update the IP address for validator node.

11.  Copy the `validator-full-node-identity.yaml`, download `genesis.blob` and `waypoint.txt` files into the same working directory on fullnode machine.

12.  Run docker-compose: `docker-compose -f docker-compose-fullnode.yaml up`.
Now you have successfully completed setting up your node.

13. Optional: if you need to block an ip address simply add it to the bottom of blocked.ips and reload haproxy
