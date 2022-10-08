---
title: "Staking Pool Operations"
slug: "staking-pool-operations"
---

# Staking Pool Operations

This document describes how to perform staking pool operations. Note that you can stake only when you met the minimal staking requirement. 

:::tip Minimum staking requirement
The current required minimum for staking is 1M APT tokens.
:::

## Initialize the stake pool

Make sure that this step was performed by the owner. See [Initialize staking pool](/nodes/validator-node/owner/index#initialize-staking-pool) in the owner documentation section.

## Joining validator set

:::tip Errors? 
If you run into any errors, see the [Issues and Workarounds](/docs/issues-and-workarounds.md).
:::

Follow these steps to setup the validator node using the operator account and join the validator set.

1. Initialize Aptos CLI.

    ```bash
    aptos init --profile mainnet-operator \
    --private-key <operator_account_private_key> \
    --rest-url https://testnet.aptoslabs.com \
    --skip-faucet
    ```
    
    :::tip
    The `account_private_key` for the operator can be found in the `private-keys.yaml` file under `~/$WORKSPACE/keys` folder.
    :::

2. Check your validator account balance. Make sure you have some coins to pay gas. You can do this step either by checking on the Aptos Explorer or using the CLI:

    On the Aptos Explorer `https://explorer.aptoslabs.com/account/<account-address>?network=testnet` or use the CLI:

    ```bash
    aptos account list --profile mainnet-operator
    ```
    
    This will show you the coin balance you have in the validator account. You will see something like:
    
    ```json
    "coin": {
        "value": "5000"
      }
    ```

3. Update validator network addresses on chain.

    ```bash
    aptos node update-validator-network-addresses  \
      --pool-address <pool-address> \
      --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
      --profile mainnet-operator
    ```

4. Rotate the validator consensus key on chain.

    ```bash
    aptos node update-consensus-key  \
      --pool-address <pool-address> \
      --operator-config-file ~/$WORKSPACE/$USERNAME/operator.yaml \
      --profile mainnet-operator
    ```

5. **Join the validator set.**

    ```bash
    aptos node join-validator-set \
      --pool-address <pool-address> \
      --profile mainnet-operator
    ```

    The `ValidatorSet` will be updated at every epoch change, which is **once every 2 hours**. You will only see your node joining the validator set in the next epoch. Both validator and fullnode will start syncing once your validator is in the validator set.

6. Check the validator set.

    ```bash
    aptos node show-validator-set --profile mainnet-operator | jq -r '.Result.pending_active' | grep <pool_address>
    ```
    
    You will see your validator node in "pending_active" list. When the next epoch change happens, the node will be moved into "active_validators" list. This will happen within one hour from the completion of previous step. **During this time you might see errors like "No connected AptosNet peers". This is normal.**
    
    ```bash
    aptos node show-validator-set --profile mainnet-operator | jq -r '.Result.active_validators' | grep <pool_address>
    ```


## Checking your stake pool information

To check the details of your stake pool, run the below CLI command with the `get-stake-pool` option by providing the `--owner-address` and `--url` fields. 

:::tip Use CLI 0.3.8 or higher
Make sure you use the CLI version 0.3.8 or higher. See [Installing Aptos CLI](/cli-tools/aptos-cli-tool/install-aptos-cli.md).
- Type `aptos --help` to see the CLI version.
- Type `aptos node get-stake-pool --help` for more on the command option for the below example.
:::

The below command is an example for Premainnet and an example owner address `e7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb`. For other networks, use the appropriate REST URL for the `--url` field. See [Aptos Blockchain Deployments](/nodes/aptos-deployments) for `--url` field values. 

```bash
aptos node get-stake-pool \
  --owner-address e7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb \
  --url https://premainnet.aptosdev.com
```

Example output:

```json
 Compiling aptos v0.3.8 (/Users/kevin/aptos-core/crates/aptos)
    Finished dev [unoptimized + debuginfo] target(s) in 32.89s
     Running `target/debug/aptos node get-stake-pool --owner-address e7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb`
{
  "Result": [
    {
      "state": "Active",
      "pool_address": "25c3482850a188d8aa6edc5751846e1226a27863643f5ebc52be4f7d822264e3",
      "operator_address": "3bec5a529b023449dfc86e9a6b5b51bf75cec4a62bf21c15bbbef08a75f7038f",
      "voter_address": "3bec5a529b023449dfc86e9a6b5b51bf75cec4a62bf21c15bbbef08a75f7038f",
      "pool_type": "StakingContract",
      "total_stake": 100525929489123,
      "commission_percentage": 10,
      "commission_not_yet_unlocked": 15949746439,
      "lockup_expiration_utc_time": "2022-10-07T07:12:55Z",
      "consensus_public_key": "0xb3a7ac1491b0165f08f136c2b02739846b6610084984d5298c2983c4f8e5553284bffca2e3fe2b99167da82717501732",
      "validator_network_addresses": [
        "/ip4/35.91.145.164/tcp/6180/noise-ik/0xeddf05470520af91b847f353dd804a04399e1213d130a4260e813527f2c49262/handshake/0"
      ],
      "fullnode_network_addresses": [],
      "epoch_info": {
        "epoch": 594,
        "epoch_interval_secs": 3600,
        "current_epoch_start_time": {
          "unix_time": 1665087178789891,
          "utc_time": "2022-10-06T20:12:58.789891Z"
        },
        "next_epoch_start_time": {
          "unix_time": 1665090778789891,
          "utc_time": "2022-10-06T21:12:58.789891Z"
        }
      }
    }
  ]
}
```


## Request commission

As an operator, you can request commission once a month, i.e., at the end of a lockup period, by running the following command. Make sure to provide the operator and the owner addresses.

```bash
aptos stake request-commission \
  --operator-address 0x3bec5a529b023449dfc86e9a6b5b51bf75cec4a62bf21c15bbbef08a75f7038f \
  --owner-address 0xe7be097a90c18f6bdd53efe0e74bf34393cac2f0ae941523ea196a47b6859edb
```

## Misc

### Checking your validator performance

To see your validator performance in the current and past epochs and rewards earned, run the below command. The output will show the validator's performance in block proposals and in governance voting and governance proposals. Default values are used in the below command. Type `aptos node analyze-validator-performance --help` to see default values used.

```bash
aptos node analyze-validator-performance --analyze-mode All \
  --url https://premainnet.aptosdev.com
```

Example output:

```json
{
  "Result": {
    "current_epoch_successful_proposals": 56,
    "current_epoch_failed_proposals": 0,
    "previous_epoch_rewards": [
      "12312716242",
      "12272043711",
      "12312912674",
      "12313011054",
      "12313109435",
      "12180092056",
      "12313305136",
      "12313403519",
      "12313501903",
      "12313600288"
    ],
    "epoch_info": {
      "epoch": 68,
      "epoch_interval": 3600000000,
      "last_epoch_start_time": {
        "unix_time": 1665074662417326,
        "utc_time": "2022-10-06T16:44:22.417326Z",
        "local_time": "Thu Oct  6 16:44:22 2022"
      },
      "next_epoch_start_time": {
        "unix_time": 1665078262417326,
        "utc_time": "2022-10-06T17:44:22.417326Z",
        "local_time": "Thu Oct  6 17:44:22 2022"
      }
    }
  }
}
```

### Rotating the consensus key

You can rotate the operator consensus key by running the following command. Make sure to provide the pool address, the path to the `config.yaml` file and the profile:

```bash
aptos node update-consensus-key \
  --pool-address <pool address> \
  --validator-config-file </path/to/config.yaml> \
  --profile <profile>
  ```

### Update addresses for validator and validator fullnode 

You can update the address for the validator node and the validator fullnode by running the following command. Make sure to provide the pool address, the path to the `operator.yaml` file and the profile:

```bash
aptos node update-validator-network-addresses \
  --pool-address <pool-address> \
  --operator-config-file </path/to/operator.yaml> \
  --profile <profile>
  ```

### Check performance for all epochs

how can we receive performance for all epochs since Genesis? The command that you have provided early, only displays info for the latest period

```bash
aptos node analyze-validator-performance --analyze-mode=detailed-epoch-table '--url=https://premainnet.aptosdev.com' --start-epoch=0 | grep <pool address>
```