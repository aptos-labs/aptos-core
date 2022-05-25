# Using Aptos Node docker
## Start Aptos Node as test mode
1. Install Docker and Docker-Compose, Aptos CLI.
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
    $ aptos genesis generate-keys --output-dir ~/$WORKSPACE
    ```

    This will create three files: `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` for you. Backup your key files somewhere safe, this is important for you to establish ownership of your node, and it will be used to claim your rewards later if eligible. Very important!!

5. Configure validator information, you need to setup a static IP / DNS address which can be used by the node, and make sure the network / firewalls are properly configured to accept external connections.

    ```
    $ aptos genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username <pick a username for your node> \
        --validator-host <Validator Node IP / DNS address>:<Port> \
        --full-node-host <Full Node IP / DNS address>:<Port>

    # for example, with IP:

    $ aptos genesis set-validator-configuration \
        --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE \
        --username aptosbot \
        --validator-host 35.232.235.205:6180 \
        --full-node-host 34.135.169.144:6182

    # for example, with DNS:

    $ aptos genesis set-validator-configuration \
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

6. Create layout YAML file, which defines the node in the validatorSet, for test mode, we can create a genesis blob containing only one node.

    ```
    $ vi layout.yaml
    ```

    Add root key, node username, and chain_id in the `layout.yaml` file, for example:

    ```
    ---
    root_key: "0x5243ca72b0766d9e9cbf2debf6153443b01a1e0e6d086c7ea206eaf6f8043956"
    users:
      - <username you created in step 5>
    chain_id: 5
    ```

7. Download AptosFramework Move bytecodes.

    Download the Aptos Framework from the release page: https://github.com/aptos-labs/aptos-core/releases/tag/aptos-framework-v0.1.0

    ```
    $ unzip framework.zip
    ```

    You should now have a folder called `framework`, which contains move bytecodes with format `.mv`.

8. Compile genesis blob and waypoint

    ```
    $ aptos genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ```

    This should create two files in your working directory, `genesis.blob` and `waypoint.txt`

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

10. Run docker-compose: `docker-compose up`.

11. [Optional] Now let's setup Fullnode on a different machine. Download the `fullnode.yaml` and `docker-compose-fullnode.yaml` configuration files into the working directory of Fullnode machine.
    ```
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/docker-compose-fullnode.yaml
    wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/aptos-node/fullnode.yaml
    ```

12. Edit `fullnode.yaml` file to update the IP address for Validator node.

13. [Optional] Copy the `validator-full-node-identity.yaml`, `genesis.blob` and `waypoint.txt` files generated above into the same working directory on Fullnode machine.

14. [Optional] Run docker-compose: `docker-compose up -f docker-compose-fullnode.yaml`.
