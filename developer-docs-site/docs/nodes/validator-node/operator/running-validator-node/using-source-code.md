---
title: "Using Source Code"
slug: "run-validator-node-using-source"
---

# Using Aptos-core source code

:::tip For validator fullnode
Use the `fullnode.yaml` to run a validator fullnode. See [Step 13](#source-code-vfn).
:::

1. Clone the Aptos repo.

      ```
      git clone https://github.com/aptos-labs/aptos-core.git
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

With your development environment ready, now you can start to setup your validator node.

5. Checkout the `testnet` branch using `git checkout --track origin/testnet`.

6. Create a directory for your Aptos node composition, and pick a username for your node. e.g.
    ```
    export WORKSPACE=testnet
    export USERNAME=alice
    mkdir ~/$WORKSPACE
    ```

:::tip Install Aptos CLI

Before proceeding further, install **Aptos CLI 0.3.1**: https://aptos.dev/cli-tools/aptos-cli-tool/install-aptos-cli 

:::

7. Generate the key pairs (node owner, voter, operator key, consensus key and networking key) in your working directory.

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

8. Configure validator information. You need to setup a static IP / DNS address (DNS is much preferred) which can be used by the node, and make sure the network / firewalls are properly configured to accept external connections.

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

9. Create a layout template file, which defines the node in the Aptos `validatorSet`. 

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

10. Build and copy the AptosFramework Move package into the `~/$WORKSPACE` directory as `framework.mrb`

    ```
    cd ~/aptos-core
    cargo run --package framework -- release
    cp head.mrb ~/$WORKSPACE/framework.mrb
    ```

11. Compile genesis blob and waypoint

    ```
    aptos genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ```

    This will create two files in your working directory, `genesis.blob` and `waypoint.txt`.

12. Copy the `validator.yaml`, `fullnode.yaml` files into this directory.
    ```
    mkdir ~/$WORKSPACE/config
    cp docker/compose/aptos-node/validator.yaml ~/$WORKSPACE/config/validator.yaml
    cp docker/compose/aptos-node/fullnode.yaml ~/$WORKSPACE/config/fullnode.yaml
    ```

    Modify the config files to update the data directory, key path, genesis file path, waypoint path.
    User must have write access to data directory.

13. <span id="source-code-vfn">To recap, in your working directory (`~/$WORKSPACE`), you should have a list of files:</span>

    - `config` folder, which includes:
      - `validator.yaml` validator config file
      - `fullnode.yaml` fullnode config file
    - `keys` folder, which includes:
      - `public-keys.yaml`: Public keys for the owner account, consensus, networking (from step 7).
      - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 7).
      - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 7).
      - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 7).
    - `username` folder, which includes: 
      - `owner.yaml`: define owner, operator, and voter mapping. They are all the same account in test mode (from step 8).
      - `operator.yaml`: Node information that will be used for both the Validator and the fullnode (from step 8). 
    - `layout.yaml`: The layout file containing the key values for root key, validator user, and chain ID (from step 9).
    - `framework.mrb`: The AptosFramework Move package (from step 10).
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 11).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validatorSet and more (from step 11).

14. Start your local Validator by running the below command:

    ```
    cargo run -p aptos-node --release -- -f ~/$WORKSPACE/config/validator.yaml
    ```

    Run fullnode in another machine:

    ```
    cargo run -p aptos-node --release -- -f ~/$WORKSPACE/config/fullnode.yaml
    ```

Now you have completed setting up your node in test mode. You can continue to our [Aptos community platform](https://community.aptoslabs.com/) website for registration.
