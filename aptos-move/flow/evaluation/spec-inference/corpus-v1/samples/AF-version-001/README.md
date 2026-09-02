# AF-version-001

This sample is a recipe over the corpus's single editable
[`framework`](framework/) package. The runner copies that package, applies
[`preparation.patch`](preparation.patch), and verifies the resulting hash before
giving the independent workspace to an agent.

## Target

- Target: `0x1::version::initialize`
- Granularity: `function`
- Original source: `aptos-move/framework/aptos-framework/sources/configs/version.move`
- Source inside the shared package: `sources/AptosFramework/configs/version.move`
- Source root: `aptos-move/framework/aptos-framework`
- Aptos Core commit: `6d836beedc56fc70c54f3b3046d1d248d850c64b`
- Shared package SHA-256: `a9681689d48bd9b0fd092b67670030c79da1d2f56df796af4a14f75d74a3b70e`
- Prepared tree SHA-256: `389387c58373fbea8bb26cb3cc413887f9f475a56f0a65d162b727c77bbd87da`
- Required contract categories: `normal-result`, `state-transition`, `frame`

Target functions:

- `initialize`

## Compilation context

The shared package contains the union of the target modules and their complete
source-level transitive module dependencies. Its module/file map and resolved
named addresses are recorded in
[`framework/corpus-modules.json`](framework/corpus-modules.json). Modules other
than this sample's target are compilation context, not additional inference
targets.

Opaque/bodyless boundaries whose contracts are visible while proving this
target. This closure traverses transparent executable callees and behavioral
predicates referenced from reached contracts:

- `0x1::system_addresses::assert_aptos_framework`

Transitive specification functions referenced by those boundary contracts:

- `0x1::signer::$address_of`
- `0x1::signer::$borrow_address`

Transitive source modules required to compile the sample:

- `0x1::account`
- `0x1::account_abstraction`
- `0x1::aggregator`
- `0x1::aggregator_factory`
- `0x1::aggregator_v2`
- `0x1::any`
- `0x1::aptos_account`
- `0x1::aptos_coin`
- `0x1::aptos_governance`
- `0x1::aptos_hash`
- `0x1::auth_data`
- `0x1::bcs`
- `0x1::bcs_stream`
- `0x1::big_ordered_map`
- `0x1::block`
- `0x1::bls12381`
- `0x1::bn254_algebra`
- `0x1::chain_id`
- `0x1::chain_status`
- `0x1::chunky_dkg`
- `0x1::chunky_dkg_config`
- `0x1::chunky_dkg_config_seqnum`
- `0x1::cmp`
- `0x1::code`
- `0x1::coin`
- `0x1::comparator`
- `0x1::confidential_amount`
- `0x1::confidential_asset`
- `0x1::confidential_balance`
- `0x1::confidential_range_proofs`
- `0x1::config_buffer`
- `0x1::consensus_config`
- `0x1::copyable_any`
- `0x1::create_signer`
- `0x1::crypto_algebra`
- `0x1::decryption`
- `0x1::delegation_pool`
- `0x1::dispatchable_fungible_asset`
- `0x1::dkg`
- `0x1::ed25519`
- `0x1::epoch_timeout_config`
- `0x1::error`
- `0x1::event`
- `0x1::execution_config`
- `0x1::features`
- `0x1::federated_keyless`
- `0x1::fixed_point32`
- `0x1::fixed_point64`
- `0x1::from_bcs`
- `0x1::function_info`
- `0x1::fungible_asset`
- `0x1::gas_schedule`
- `0x1::genesis`
- `0x1::governance_proposal`
- `0x1::guid`
- `0x1::hash`
- `0x1::init`
- `0x1::jwk_consensus_config`
- `0x1::jwks`
- `0x1::keyless`
- `0x1::keyless_account`
- `0x1::math128`
- `0x1::math64`
- `0x1::math_fixed64`
- `0x1::mem`
- `0x1::multi_ed25519`
- `0x1::multi_key`
- `0x1::multisig_account`
- `0x1::nonce_validation`
- `0x1::object`
- `0x1::option`
- `0x1::optional_aggregator`
- `0x1::ordered_map`
- `0x1::pool_u64`
- `0x1::pool_u64_unbound`
- `0x1::primary_fungible_store`
- `0x1::randomness`
- `0x1::randomness_api_v0_config`
- `0x1::randomness_config`
- `0x1::randomness_config_seqnum`
- `0x1::reconfiguration`
- `0x1::reconfiguration_state`
- `0x1::reconfiguration_with_dkg`
- `0x1::reflect`
- `0x1::resource_account`
- `0x1::result`
- `0x1::ristretto255`
- `0x1::ristretto255_bulletproofs`
- `0x1::ristretto255_pedersen`
- `0x1::secp256k1`
- `0x1::secp256r1`
- `0x1::sigma_protocol`
- `0x1::sigma_protocol_fiat_shamir`
- `0x1::sigma_protocol_homomorphism`
- `0x1::sigma_protocol_key_rotation`
- `0x1::sigma_protocol_proof`
- `0x1::sigma_protocol_registration`
- `0x1::sigma_protocol_representation`
- `0x1::sigma_protocol_representation_vec`
- `0x1::sigma_protocol_statement`
- `0x1::sigma_protocol_statement_builder`
- `0x1::sigma_protocol_transfer`
- `0x1::sigma_protocol_utils`
- `0x1::sigma_protocol_withdraw`
- `0x1::sigma_protocol_witness`
- `0x1::signer`
- `0x1::simple_map`
- `0x1::single_key`
- `0x1::smart_table`
- `0x1::stake`
- `0x1::staking_config`
- `0x1::staking_contract`
- `0x1::state_storage`
- `0x1::storage_gas`
- `0x1::storage_slots_allocator`
- `0x1::string`
- `0x1::string_utils`
- `0x1::system_addresses`
- `0x1::table`
- `0x1::table_with_length`
- `0x1::timestamp`
- `0x1::transaction_context`
- `0x1::transaction_fee`
- `0x1::transaction_limits`
- `0x1::transaction_validation`
- `0x1::type_info`
- `0x1::util`
- `0x1::validator_consensus_info`
- `0x1::vector`
- `0x1::vesting`
- `0x1::voting`

## Preparation

The executable Move implementation is unchanged. Existing target reference
blocks removed from the agent-visible source are:

- `sources/AptosFramework/configs/version.spec.move`: `initialize` (1 block(s))

The reproducible transformation is [`preparation.patch`](preparation.patch).
The agent may edit only:

- `sources/AptosFramework/configs/version.move`
- `sources/AptosFramework/configs/version.spec.move`
