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

3. Download the validator.yaml and docker-compose.yaml configuration files into this directory.
    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/docker-compose.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/validator.yaml
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

6. Create a layout template file, which defines the node in the Aptos `validatorSet`. 

  ```
  aptos genesis generate-layout-template --output-file ~/$WORKSPACE/layout.yaml
  ```
  Edit the `layout.yaml`, add the `root_key`, the validator node username, and `chain_id`:

  ```
  root_key: "D04470F43AB6AEAA4EB616B72128881EEF77346F2075FFE68E14BA7DEBD8095E"
  users: ["<username you specified from previous step>"]
  chain_id: 43
  allow_new_validators: false
  epoch_duration_secs: 7200
  is_test: true
  min_stake: 100000000000000
  min_voting_threshold: 100000000000000
  max_stake: 100000000000000000
  recurring_lockup_duration_secs: 86400
  required_proposer_stake: 100000000000000
  rewards_apy_percentage: 10
  voting_duration_secs: 43200
  voting_power_increase_limit: 20
  ```

  Please make sure you use the same root public key as shown in the example and same chain ID, those config will be used during registration to verify your node.

7. Download the AptosFramework Move package into the `~/$WORKSPACE` directory as `framework.mrb`

    ```
    wget https://github.com/aptos-labs/aptos-core/releases/download/aptos-framework-v0.3.0/framework.mrb -P ~/$WORKSPACE
    ```

8. Compile genesis blob and waypoint

    ```
    aptos genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ```

    This will create two files in your working directory, `genesis.blob` and `waypoint.txt`.

9. <span id="docker-files">To recap, in your working directory, you should have a list of files:</span>

    - `docker-compose.yaml` docker compose file to run validator and fullnode
    - `keys` folder, which includes:
      - `public-keys.yaml`: Public keys for the owner account, consensus, networking (from step 4).
      - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 4).
      - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 4).
      - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 4).
    - `username` folder, which includes: 
      - `owner.yaml`: define owner, operator, and voter mapping. They are all the same account in test mode (from step 5).
      - `operator.yaml`: Node information that will be used for both the Validator and the fullnode (from step 5). 
    - `layout.yaml`: The layout file containing the key values for root key, validator user, and chain ID (from step 6).
    - `framework.mrb`: The AptosFramework Move package (from step 7).
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 8).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validatorSet and more (from step 8).

10. Run docker-compose: `docker-compose up`. (or `docker compose up` depends on your version)

Now you have completed setting up your validator node in test mode. You can continue to our [Aptos community platform](https://community.aptoslabs.com/) website for registration. Additionally, you can also setup a fullnode following the instructions below.

11.  [Optional] <span id="docker-vfn">Now let's setup fullnode on a different machine. Download the `fullnode.yaml` and `docker-compose-fullnode.yaml` configuration files into the working directory of fullnode machine.</span>

    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/docker-compose-fullnode.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/fullnode.yaml
    ```

12.  Edit `fullnode.yaml` file to update the IP address for validator node.

13.  [Optional] Copy the `validator-full-node-identity.yaml`, `genesis.blob` and `waypoint.txt` files generated above into the same working directory on fullnode machine.

14.  [Optional] Run docker-compose: `docker-compose -f docker-compose-fullnode.yaml up`.
Now you have successfully completed setting up your node in test mode. You can now proceed to the [Aptos community platform](https://community.aptoslabs.com/) website for registration.
