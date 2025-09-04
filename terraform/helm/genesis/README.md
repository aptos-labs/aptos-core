# velor-genesis

![Version: 0.1.0](https://img.shields.io/badge/Version-0.1.0-informational?style=flat-square) ![AppVersion: 0.1.0](https://img.shields.io/badge/AppVersion-0.1.0-informational?style=flat-square)

Velor blockchain automated genesis ceremony for testnets

**Homepage:** <https://velorlabs.com/>

## Source Code

* <https://github.com/velor-chain/velor-core>

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| chain.allow_new_validators | bool | `false` | Allow new validators to join after genesis |
| chain.chain_id | int | `4` | Velor Chain ID |
| chain.epoch_duration_secs | int | `7200` | Length of each epoch in seconds. Defaults to 2 hours |
| chain.era | int | `1` | Internal: Bump this number to wipe the underlying storage |
| chain.is_test | bool | `true` | If true, genesis will create a resources account that can mint coins. |
| chain.max_stake | int | `100000000000000000` | Maximum stake. Defaults to 1B VELOR coins with 8 decimals |
| chain.min_price_per_gas_unit | int | `1` | Minimum price per gas unit |
| chain.min_stake | int | `100000000000000` | Minimum stake. Defaults to 1M VELOR coins with 8 decimals |
| chain.min_voting_threshold | int | `100000000000000` | Mininum voting threshold. Defaults to 1M VELOR coins with 8 decimals |
| chain.name | string | `"testnet"` | Internal: name of the testnet to connect to |
| chain.on_chain_consensus_config | string | `nil` | Onchain Consensus Config |
| chain.on_chain_execution_config | string | `nil` | Onchain Execution Config |
| chain.recurring_lockup_duration_secs | int | `86400` | Recurring lockup duration in seconds. Defaults to 1 day |
| chain.required_proposer_stake | int | `100000000000000` | Required stake to be a proposer. 1M VELOR coins with 8 decimals |
| chain.rewards_apy_percentage | int | `10` | Rewards APY percentage |
| chain.root_key | string | `"0x5243ca72b0766d9e9cbf2debf6153443b01a1e0e6d086c7ea206eaf6f8043956"` | If specified, the key for the minting capability in testnet |
| chain.voting_duration_secs | int | `43200` | Voting duration in seconds. Defaults to 12 hours |
| chain.voting_power_increase_limit | int | `20` | Limit on how much voting power can join every epoch. Defaults to 20%. |
| enabled | bool | `true` | Used to toggle on and off the automatic genesis job |
| genesis.cluster_name | string | `"unknown"` |  |
| genesis.domain | string | `nil` | If set, the base domain name of the fullnode and validator endpoints |
| genesis.fullnode.enable_onchain_discovery | bool | `true` | Use External DNS as created by velor-node helm chart for fullnode host in genesis |
| genesis.fullnode.internal_host_suffix | string | `"fullnode-lb"` | If `enable_onchain_discovery` is false, use this host suffix for internal kubernetes service name |
| genesis.genesis_blob_upload_url | string | `"https://us-west1-velor-forge-gcp-0.cloudfunctions.net/signed-url"` |  |
| genesis.image.pullPolicy | string | `"IfNotPresent"` | Image pull policy to use for tools image |
| genesis.image.repo | string | `"velorlabs/tools"` | Image repo to use for tools image for running genesis |
| genesis.image.tag | string | `nil` | Image tag to use for tools image. If set, overrides `imageTag` |
| genesis.moveModulesDir | string | `"/velor-framework/move/modules"` | The local path for move modules in the docker image. Defaults to the velor-framework in the velorlabs/tools docker image |
| genesis.multicluster | object | `{"domain_suffixes":"","enabled":false}` | Options for multicluster mode. This is *experimental only* |
| genesis.numValidators | int | `1` | Number of validators to include in genesis |
| genesis.username_prefix | string | `"velor-node"` | If `enable_onchain_discovery` is false, use this kubernetes service name prefix. It should be the fullname for the velor-node helm release |
| genesis.validator.enable_onchain_discovery | bool | `false` | Use External DNS as created by velor-node helm chart for validator host in genesis |
| genesis.validator.internal_host_suffix | string | `"validator-lb"` | If `enable_onchain_discovery` is false, use this host suffix for internal kubernetes service name |
| genesis.validator.key_seed | string | `nil` | Random seed to generate validator keys in order to make the key generation deterministic |
| genesis.validator.larger_stake_amount | string | `"1000000000000000"` | Stake amount for nodes we are giving larger state to. Defaults to 10M VELOR coins with 8 decimals |
| genesis.validator.num_validators_with_larger_stake | int | `0` | Number of validators to give larger stake in genesis to. |
| genesis.validator.stake_amount | string | `"100000000000000"` | Stake amount for each validator in this testnet. Defaults to 1M VELOR coins with 8 decimals |
| imageTag | string | `"testnet"` | Default image tag to use for all tools images |
| labels | string | `nil` |  |
| serviceAccount.create | bool | `true` | Specifies whether a service account should be created |
| serviceAccount.name | string | `nil` | The name of the service account to use. If not set and create is true, a name is generated using the fullname template |

----------------------------------------------
Autogenerated from chart metadata using [helm-docs v1.14.2](https://github.com/norwoodj/helm-docs/releases/v1.14.2)
