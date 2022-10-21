
<a name="@Aptos_Token_Framework_0"></a>

# Aptos Token Framework


This is the reference documentation of the Aptos Token framework.


<a name="@Index_1"></a>

## Index


-  [`0x1::account`](../../aptos-framework/doc/account.md#0x1_account)
-  [`0x1::aggregator`](../../aptos-framework/doc/aggregator.md#0x1_aggregator)
-  [`0x1::aggregator_factory`](../../aptos-framework/doc/aggregator_factory.md#0x1_aggregator_factory)
-  [`0x1::any`](../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any)
-  [`0x1::aptos_account`](../../aptos-framework/doc/aptos_account.md#0x1_aptos_account)
-  [`0x1::aptos_coin`](../../aptos-framework/doc/aptos_coin.md#0x1_aptos_coin)
-  [`0x1::aptos_governance`](../../aptos-framework/doc/aptos_governance.md#0x1_aptos_governance)
-  [`0x1::bcs`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs)
-  [`0x1::block`](../../aptos-framework/doc/block.md#0x1_block)
-  [`0x1::bls12381`](../../aptos-framework/../aptos-stdlib/doc/bls12381.md#0x1_bls12381)
-  [`0x1::chain_id`](../../aptos-framework/doc/chain_id.md#0x1_chain_id)
-  [`0x1::chain_status`](../../aptos-framework/doc/chain_status.md#0x1_chain_status)
-  [`0x1::code`](../../aptos-framework/doc/code.md#0x1_code)
-  [`0x1::coin`](../../aptos-framework/doc/coin.md#0x1_coin)
-  [`0x1::comparator`](../../aptos-framework/../aptos-stdlib/doc/comparator.md#0x1_comparator)
-  [`0x1::consensus_config`](../../aptos-framework/doc/consensus_config.md#0x1_consensus_config)
-  [`0x1::copyable_any`](../../aptos-framework/../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any)
-  [`0x1::ed25519`](../../aptos-framework/../aptos-stdlib/doc/ed25519.md#0x1_ed25519)
-  [`0x1::error`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error)
-  [`0x1::event`](../../aptos-framework/doc/event.md#0x1_event)
-  [`0x1::features`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features)
-  [`0x1::fixed_point32`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/fixed_point32.md#0x1_fixed_point32)
-  [`0x1::from_bcs`](../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs)
-  [`0x1::gas_schedule`](../../aptos-framework/doc/gas_schedule.md#0x1_gas_schedule)
-  [`0x1::genesis`](../../aptos-framework/doc/genesis.md#0x1_genesis)
-  [`0x1::governance_proposal`](../../aptos-framework/doc/governance_proposal.md#0x1_governance_proposal)
-  [`0x1::guid`](../../aptos-framework/doc/guid.md#0x1_guid)
-  [`0x1::hash`](../../aptos-framework/../aptos-stdlib/doc/hash.md#0x1_hash)
-  [`0x1::math64`](../../aptos-framework/../aptos-stdlib/doc/math64.md#0x1_math64)
-  [`0x1::multi_ed25519`](../../aptos-framework/../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519)
-  [`0x1::option`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option)
-  [`0x1::optional_aggregator`](../../aptos-framework/doc/optional_aggregator.md#0x1_optional_aggregator)
-  [`0x1::pool_u64`](../../aptos-framework/../aptos-stdlib/doc/pool_u64.md#0x1_pool_u64)
-  [`0x3::property_map`](property_map.md#0x3_property_map)
-  [`0x1::reconfiguration`](../../aptos-framework/doc/reconfiguration.md#0x1_reconfiguration)
-  [`0x1::resource_account`](../../aptos-framework/doc/resource_account.md#0x1_resource_account)
-  [`0x1::signer`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer)
-  [`0x1::simple_map`](../../aptos-framework/../aptos-stdlib/doc/simple_map.md#0x1_simple_map)
-  [`0x1::stake`](../../aptos-framework/doc/stake.md#0x1_stake)
-  [`0x1::staking_config`](../../aptos-framework/doc/staking_config.md#0x1_staking_config)
-  [`0x1::staking_contract`](../../aptos-framework/doc/staking_contract.md#0x1_staking_contract)
-  [`0x1::state_storage`](../../aptos-framework/doc/state_storage.md#0x1_state_storage)
-  [`0x1::storage_gas`](../../aptos-framework/doc/storage_gas.md#0x1_storage_gas)
-  [`0x1::string`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string)
-  [`0x1::system_addresses`](../../aptos-framework/doc/system_addresses.md#0x1_system_addresses)
-  [`0x1::table`](../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table)
-  [`0x1::table_with_length`](../../aptos-framework/../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length)
-  [`0x1::timestamp`](../../aptos-framework/doc/timestamp.md#0x1_timestamp)
-  [`0x3::token`](token.md#0x3_token)
-  [`0x3::token_coin_swap`](token_coin_swap.md#0x3_token_coin_swap)
-  [`0x3::token_transfers`](token_transfers.md#0x3_token_transfers)
-  [`0x1::transaction_context`](../../aptos-framework/doc/transaction_context.md#0x1_transaction_context)
-  [`0x1::transaction_fee`](../../aptos-framework/doc/transaction_fee.md#0x1_transaction_fee)
-  [`0x1::transaction_validation`](../../aptos-framework/doc/transaction_validation.md#0x1_transaction_validation)
-  [`0x1::type_info`](../../aptos-framework/../aptos-stdlib/doc/type_info.md#0x1_type_info)
-  [`0x1::util`](../../aptos-framework/doc/util.md#0x1_util)
-  [`0x1::vector`](../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector)
-  [`0x1::version`](../../aptos-framework/doc/version.md#0x1_version)
-  [`0x1::vesting`](../../aptos-framework/doc/vesting.md#0x1_vesting)
-  [`0x1::voting`](../../aptos-framework/doc/voting.md#0x1_voting)


[move-book]: https://move-language.github.io/move/introduction.html
