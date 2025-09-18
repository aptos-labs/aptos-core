
<a id="0x1_features"></a>

# Module `0x1::features`

Defines feature flags for Supra. Those are used in Supra specific implementations of features in
the Move stdlib, the Supra stdlib, and the Supra framework.

============================================================================================
Feature Flag Definitions

Each feature flag should come with documentation which justifies the need of the flag.
Introduction of a new feature flag requires approval of framework owners. Be frugal when
introducing new feature flags, as too many can make it hard to understand the code.

Each feature flag should come with a specification of a lifetime:

- a *transient* feature flag is only needed until a related code rollout has happened. This
is typically associated with the introduction of new native Move functions, and is only used
from Move code. The owner of this feature is obliged to remove it once this can be done.

- a *permanent* feature flag is required to stay around forever. Typically, those flags guard
behavior in native code, and the behavior with or without the feature need to be preserved
for playback.

Note that removing a feature flag still requires the function which tests for the feature
(like <code>code_dependency_check_enabled</code> below) to stay around for compatibility reasons, as it
is a public function. However, once the feature flag is disabled, those functions can constantly
return true.


-  [Resource `Features`](#0x1_features_Features)
-  [Resource `PendingFeatures`](#0x1_features_PendingFeatures)
-  [Constants](#@Constants_0)
-  [Function `code_dependency_check_enabled`](#0x1_features_code_dependency_check_enabled)
-  [Function `treat_friend_as_private`](#0x1_features_treat_friend_as_private)
-  [Function `get_sha_512_and_ripemd_160_feature`](#0x1_features_get_sha_512_and_ripemd_160_feature)
-  [Function `sha_512_and_ripemd_160_enabled`](#0x1_features_sha_512_and_ripemd_160_enabled)
-  [Function `get_aptos_stdlib_chain_id_feature`](#0x1_features_get_aptos_stdlib_chain_id_feature)
-  [Function `aptos_stdlib_chain_id_enabled`](#0x1_features_aptos_stdlib_chain_id_enabled)
-  [Function `get_vm_binary_format_v6`](#0x1_features_get_vm_binary_format_v6)
-  [Function `allow_vm_binary_format_v6`](#0x1_features_allow_vm_binary_format_v6)
-  [Function `get_collect_and_distribute_gas_fees_feature`](#0x1_features_get_collect_and_distribute_gas_fees_feature)
-  [Function `collect_and_distribute_gas_fees`](#0x1_features_collect_and_distribute_gas_fees)
-  [Function `multi_ed25519_pk_validate_v2_feature`](#0x1_features_multi_ed25519_pk_validate_v2_feature)
-  [Function `multi_ed25519_pk_validate_v2_enabled`](#0x1_features_multi_ed25519_pk_validate_v2_enabled)
-  [Function `get_blake2b_256_feature`](#0x1_features_get_blake2b_256_feature)
-  [Function `blake2b_256_enabled`](#0x1_features_blake2b_256_enabled)
-  [Function `get_resource_groups_feature`](#0x1_features_get_resource_groups_feature)
-  [Function `resource_groups_enabled`](#0x1_features_resource_groups_enabled)
-  [Function `get_multisig_accounts_feature`](#0x1_features_get_multisig_accounts_feature)
-  [Function `multisig_accounts_enabled`](#0x1_features_multisig_accounts_enabled)
-  [Function `get_delegation_pools_feature`](#0x1_features_get_delegation_pools_feature)
-  [Function `delegation_pools_enabled`](#0x1_features_delegation_pools_enabled)
-  [Function `get_cryptography_algebra_natives_feature`](#0x1_features_get_cryptography_algebra_natives_feature)
-  [Function `cryptography_algebra_enabled`](#0x1_features_cryptography_algebra_enabled)
-  [Function `get_bls12_381_strutures_feature`](#0x1_features_get_bls12_381_strutures_feature)
-  [Function `bls12_381_structures_enabled`](#0x1_features_bls12_381_structures_enabled)
-  [Function `get_periodical_reward_rate_decrease_feature`](#0x1_features_get_periodical_reward_rate_decrease_feature)
-  [Function `periodical_reward_rate_decrease_enabled`](#0x1_features_periodical_reward_rate_decrease_enabled)
-  [Function `get_partial_governance_voting`](#0x1_features_get_partial_governance_voting)
-  [Function `partial_governance_voting_enabled`](#0x1_features_partial_governance_voting_enabled)
-  [Function `get_delegation_pool_partial_governance_voting`](#0x1_features_get_delegation_pool_partial_governance_voting)
-  [Function `delegation_pool_partial_governance_voting_enabled`](#0x1_features_delegation_pool_partial_governance_voting_enabled)
-  [Function `fee_payer_enabled`](#0x1_features_fee_payer_enabled)
-  [Function `get_auids`](#0x1_features_get_auids)
-  [Function `auids_enabled`](#0x1_features_auids_enabled)
-  [Function `get_bulletproofs_feature`](#0x1_features_get_bulletproofs_feature)
-  [Function `bulletproofs_enabled`](#0x1_features_bulletproofs_enabled)
-  [Function `get_signer_native_format_fix_feature`](#0x1_features_get_signer_native_format_fix_feature)
-  [Function `signer_native_format_fix_enabled`](#0x1_features_signer_native_format_fix_enabled)
-  [Function `get_module_event_feature`](#0x1_features_get_module_event_feature)
-  [Function `module_event_enabled`](#0x1_features_module_event_enabled)
-  [Function `get_aggregator_v2_api_feature`](#0x1_features_get_aggregator_v2_api_feature)
-  [Function `aggregator_v2_api_enabled`](#0x1_features_aggregator_v2_api_enabled)
-  [Function `get_aggregator_snapshots_feature`](#0x1_features_get_aggregator_snapshots_feature)
-  [Function `aggregator_snapshots_enabled`](#0x1_features_aggregator_snapshots_enabled)
-  [Function `get_sponsored_automatic_account_creation`](#0x1_features_get_sponsored_automatic_account_creation)
-  [Function `sponsored_automatic_account_creation_enabled`](#0x1_features_sponsored_automatic_account_creation_enabled)
-  [Function `get_concurrent_token_v2_feature`](#0x1_features_get_concurrent_token_v2_feature)
-  [Function `concurrent_token_v2_enabled`](#0x1_features_concurrent_token_v2_enabled)
-  [Function `get_concurrent_assets_feature`](#0x1_features_get_concurrent_assets_feature)
-  [Function `concurrent_assets_enabled`](#0x1_features_concurrent_assets_enabled)
-  [Function `get_operator_beneficiary_change_feature`](#0x1_features_get_operator_beneficiary_change_feature)
-  [Function `operator_beneficiary_change_enabled`](#0x1_features_operator_beneficiary_change_enabled)
-  [Function `get_commission_change_delegation_pool_feature`](#0x1_features_get_commission_change_delegation_pool_feature)
-  [Function `commission_change_delegation_pool_enabled`](#0x1_features_commission_change_delegation_pool_enabled)
-  [Function `get_bn254_strutures_feature`](#0x1_features_get_bn254_strutures_feature)
-  [Function `bn254_structures_enabled`](#0x1_features_bn254_structures_enabled)
-  [Function `get_reconfigure_with_dkg_feature`](#0x1_features_get_reconfigure_with_dkg_feature)
-  [Function `reconfigure_with_dkg_enabled`](#0x1_features_reconfigure_with_dkg_enabled)
-  [Function `get_keyless_accounts_feature`](#0x1_features_get_keyless_accounts_feature)
-  [Function `keyless_accounts_enabled`](#0x1_features_keyless_accounts_enabled)
-  [Function `get_keyless_but_zkless_accounts_feature`](#0x1_features_get_keyless_but_zkless_accounts_feature)
-  [Function `keyless_but_zkless_accounts_feature_enabled`](#0x1_features_keyless_but_zkless_accounts_feature_enabled)
-  [Function `get_jwk_consensus_feature`](#0x1_features_get_jwk_consensus_feature)
-  [Function `jwk_consensus_enabled`](#0x1_features_jwk_consensus_enabled)
-  [Function `get_concurrent_fungible_assets_feature`](#0x1_features_get_concurrent_fungible_assets_feature)
-  [Function `concurrent_fungible_assets_enabled`](#0x1_features_concurrent_fungible_assets_enabled)
-  [Function `is_object_code_deployment_enabled`](#0x1_features_is_object_code_deployment_enabled)
-  [Function `get_max_object_nesting_check_feature`](#0x1_features_get_max_object_nesting_check_feature)
-  [Function `max_object_nesting_check_enabled`](#0x1_features_max_object_nesting_check_enabled)
-  [Function `get_keyless_accounts_with_passkeys_feature`](#0x1_features_get_keyless_accounts_with_passkeys_feature)
-  [Function `keyless_accounts_with_passkeys_feature_enabled`](#0x1_features_keyless_accounts_with_passkeys_feature_enabled)
-  [Function `get_multisig_v2_enhancement_feature`](#0x1_features_get_multisig_v2_enhancement_feature)
-  [Function `multisig_v2_enhancement_feature_enabled`](#0x1_features_multisig_v2_enhancement_feature_enabled)
-  [Function `get_delegation_pool_allowlisting_feature`](#0x1_features_get_delegation_pool_allowlisting_feature)
-  [Function `delegation_pool_allowlisting_enabled`](#0x1_features_delegation_pool_allowlisting_enabled)
-  [Function `get_module_event_migration_feature`](#0x1_features_get_module_event_migration_feature)
-  [Function `module_event_migration_enabled`](#0x1_features_module_event_migration_enabled)
-  [Function `get_transaction_context_extension_feature`](#0x1_features_get_transaction_context_extension_feature)
-  [Function `transaction_context_extension_enabled`](#0x1_features_transaction_context_extension_enabled)
-  [Function `get_coin_to_fungible_asset_migration_feature`](#0x1_features_get_coin_to_fungible_asset_migration_feature)
-  [Function `coin_to_fungible_asset_migration_feature_enabled`](#0x1_features_coin_to_fungible_asset_migration_feature_enabled)
-  [Function `get_primary_apt_fungible_store_at_user_address_feature`](#0x1_features_get_primary_apt_fungible_store_at_user_address_feature)
-  [Function `primary_apt_fungible_store_at_user_address_enabled`](#0x1_features_primary_apt_fungible_store_at_user_address_enabled)
-  [Function `aggregator_v2_is_at_least_api_enabled`](#0x1_features_aggregator_v2_is_at_least_api_enabled)
-  [Function `get_object_native_derived_address_feature`](#0x1_features_get_object_native_derived_address_feature)
-  [Function `object_native_derived_address_enabled`](#0x1_features_object_native_derived_address_enabled)
-  [Function `get_dispatchable_fungible_asset_feature`](#0x1_features_get_dispatchable_fungible_asset_feature)
-  [Function `dispatchable_fungible_asset_enabled`](#0x1_features_dispatchable_fungible_asset_enabled)
-  [Function `get_new_accounts_default_to_fa_supra_store_feature`](#0x1_features_get_new_accounts_default_to_fa_supra_store_feature)
-  [Function `new_accounts_default_to_fa_supra_store_enabled`](#0x1_features_new_accounts_default_to_fa_supra_store_enabled)
-  [Function `get_operations_default_to_fa_supra_store_feature`](#0x1_features_get_operations_default_to_fa_supra_store_feature)
-  [Function `operations_default_to_fa_supra_store_enabled`](#0x1_features_operations_default_to_fa_supra_store_enabled)
-  [Function `get_concurrent_fungible_balance_feature`](#0x1_features_get_concurrent_fungible_balance_feature)
-  [Function `concurrent_fungible_balance_enabled`](#0x1_features_concurrent_fungible_balance_enabled)
-  [Function `get_default_to_concurrent_fungible_balance_feature`](#0x1_features_get_default_to_concurrent_fungible_balance_feature)
-  [Function `default_to_concurrent_fungible_balance_enabled`](#0x1_features_default_to_concurrent_fungible_balance_enabled)
-  [Function `get_abort_if_multisig_payload_mismatch_feature`](#0x1_features_get_abort_if_multisig_payload_mismatch_feature)
-  [Function `abort_if_multisig_payload_mismatch_enabled`](#0x1_features_abort_if_multisig_payload_mismatch_enabled)
-  [Function `get_supra_native_automation_feature`](#0x1_features_get_supra_native_automation_feature)
-  [Function `supra_native_automation_enabled`](#0x1_features_supra_native_automation_enabled)
-  [Function `get_supra_eth_trie_feature`](#0x1_features_get_supra_eth_trie_feature)
-  [Function `supra_eth_trie_enabled`](#0x1_features_supra_eth_trie_enabled)
-  [Function `get_supra_automation_payload_gas_check_feature`](#0x1_features_get_supra_automation_payload_gas_check_feature)
-  [Function `supra_automation_payload_gas_check_enabled`](#0x1_features_supra_automation_payload_gas_check_enabled)
-  [Function `get_supra_private_poll_feature`](#0x1_features_get_supra_private_poll_feature)
-  [Function `supra_private_poll_enabled`](#0x1_features_supra_private_poll_enabled)
-  [Function `get_supra_automation_task_sync_feature`](#0x1_features_get_supra_automation_task_sync_feature)
-  [Function `supra_automation_task_sync_enabled`](#0x1_features_supra_automation_task_sync_enabled)
-  [Function `get_supra_count_failed_proposals_feature`](#0x1_features_get_supra_count_failed_proposals_feature)
-  [Function `supra_count_failed_proposals_enabled`](#0x1_features_supra_count_failed_proposals_enabled)
-  [Function `get_supra_rlp_feature`](#0x1_features_get_supra_rlp_feature)
-  [Function `supra_rlp_enabled`](#0x1_features_supra_rlp_enabled)
-  [Function `change_feature_flags`](#0x1_features_change_feature_flags)
-  [Function `change_feature_flags_internal`](#0x1_features_change_feature_flags_internal)
-  [Function `change_feature_flags_for_next_epoch`](#0x1_features_change_feature_flags_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_features_on_new_epoch)
-  [Function `is_enabled`](#0x1_features_is_enabled)
-  [Function `set`](#0x1_features_set)
-  [Function `contains`](#0x1_features_contains)
-  [Function `apply_diff`](#0x1_features_apply_diff)
-  [Function `ensure_framework_signer`](#0x1_features_ensure_framework_signer)
-  [Function `change_feature_flags_for_verification`](#0x1_features_change_feature_flags_for_verification)
-  [Specification](#@Specification_1)
    -  [Resource `Features`](#@Specification_1_Features)
    -  [Resource `PendingFeatures`](#@Specification_1_PendingFeatures)
    -  [Function `periodical_reward_rate_decrease_enabled`](#@Specification_1_periodical_reward_rate_decrease_enabled)
    -  [Function `partial_governance_voting_enabled`](#@Specification_1_partial_governance_voting_enabled)
    -  [Function `module_event_enabled`](#@Specification_1_module_event_enabled)
    -  [Function `abort_if_multisig_payload_mismatch_enabled`](#@Specification_1_abort_if_multisig_payload_mismatch_enabled)
    -  [Function `change_feature_flags_internal`](#@Specification_1_change_feature_flags_internal)
    -  [Function `change_feature_flags_for_next_epoch`](#@Specification_1_change_feature_flags_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)
    -  [Function `is_enabled`](#@Specification_1_is_enabled)
    -  [Function `set`](#@Specification_1_set)
    -  [Function `contains`](#@Specification_1_contains)
    -  [Function `apply_diff`](#@Specification_1_apply_diff)


<pre><code><b>use</b> <a href="error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_features_Features"></a>

## Resource `Features`

The enabled features, represented by a bitset stored on chain.


<pre><code><b>struct</b> <a href="features.md#0x1_features_Features">Features</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="features.md#0x1_features">features</a>: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_features_PendingFeatures"></a>

## Resource `PendingFeatures`

This resource holds the feature vec updates received in the current epoch.
On epoch change, the updates take effect and this buffer is cleared.


<pre><code><b>struct</b> <a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="features.md#0x1_features">features</a>: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_features_ABORT_IF_MULTISIG_PAYLOAD_MISMATCH"></a>

Whether the multisig v2 fix is enabled. Once enabled, the multisig transaction execution will explicitly
abort if the provided payload does not match the payload stored on-chain.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_ABORT_IF_MULTISIG_PAYLOAD_MISMATCH">ABORT_IF_MULTISIG_PAYLOAD_MISMATCH</a>: u64 = 70;
</code></pre>



<a id="0x1_features_AGGREGATOR_V2_IS_AT_LEAST_API"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_AGGREGATOR_V2_IS_AT_LEAST_API">AGGREGATOR_V2_IS_AT_LEAST_API</a>: u64 = 66;
</code></pre>



<a id="0x1_features_APTOS_STD_CHAIN_ID_NATIVES"></a>

Whether the new <code>aptos_stdlib::type_info::chain_id()</code> native for fetching the chain ID is enabled.
This is needed because of the introduction of a new native function.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_APTOS_STD_CHAIN_ID_NATIVES">APTOS_STD_CHAIN_ID_NATIVES</a>: u64 = 4;
</code></pre>



<a id="0x1_features_APTOS_UNIQUE_IDENTIFIERS"></a>

Whether enable MOVE functions to call create_auid method to create AUIDs.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_APTOS_UNIQUE_IDENTIFIERS">APTOS_UNIQUE_IDENTIFIERS</a>: u64 = 23;
</code></pre>



<a id="0x1_features_BLAKE2B_256_NATIVE"></a>

Whether the new BLAKE2B-256 hash function native is enabled.
This is needed because of the introduction of new native function(s).
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_BLAKE2B_256_NATIVE">BLAKE2B_256_NATIVE</a>: u64 = 8;
</code></pre>



<a id="0x1_features_BLS12_381_STRUCTURES"></a>

Whether the generic algebra implementation for BLS12381 operations are enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_BLS12_381_STRUCTURES">BLS12_381_STRUCTURES</a>: u64 = 13;
</code></pre>



<a id="0x1_features_BN254_STRUCTURES"></a>

Whether the generic algebra implementation for BN254 operations are enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_BN254_STRUCTURES">BN254_STRUCTURES</a>: u64 = 43;
</code></pre>



<a id="0x1_features_BULLETPROOFS_NATIVES"></a>

Whether the Bulletproofs zero-knowledge range proof module is enabled, and the related native function is
available. This is needed because of the introduction of a new native function.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_BULLETPROOFS_NATIVES">BULLETPROOFS_NATIVES</a>: u64 = 24;
</code></pre>



<a id="0x1_features_CHARGE_INVARIANT_VIOLATION"></a>

Charge invariant violation error.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_CHARGE_INVARIANT_VIOLATION">CHARGE_INVARIANT_VIOLATION</a>: u64 = 20;
</code></pre>



<a id="0x1_features_CODE_DEPENDENCY_CHECK"></a>

Whether validation of package dependencies is enabled, and the related native function is
available. This is needed because of introduction of a new native function.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_CODE_DEPENDENCY_CHECK">CODE_DEPENDENCY_CHECK</a>: u64 = 1;
</code></pre>



<a id="0x1_features_COIN_TO_FUNGIBLE_ASSET_MIGRATION"></a>

Whether migration from coin to fungible asset feature is enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_COIN_TO_FUNGIBLE_ASSET_MIGRATION">COIN_TO_FUNGIBLE_ASSET_MIGRATION</a>: u64 = 60;
</code></pre>



<a id="0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES"></a>

Whether gas fees are collected and distributed to the block proposers.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES">COLLECT_AND_DISTRIBUTE_GAS_FEES</a>: u64 = 6;
</code></pre>



<a id="0x1_features_COMMISSION_CHANGE_DELEGATION_POOL"></a>

Whether the operator commission rate change in delegation pool is enabled.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_COMMISSION_CHANGE_DELEGATION_POOL">COMMISSION_CHANGE_DELEGATION_POOL</a>: u64 = 42;
</code></pre>



<a id="0x1_features_CONCURRENT_FUNGIBLE_ASSETS"></a>

Whether enable Fungible Asset creation
to create higher throughput concurrent variants.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_CONCURRENT_FUNGIBLE_ASSETS">CONCURRENT_FUNGIBLE_ASSETS</a>: u64 = 50;
</code></pre>



<a id="0x1_features_CONCURRENT_FUNGIBLE_BALANCE"></a>

Whether enable concurent Fungible Balance
to create higher throughput concurrent variants.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_CONCURRENT_FUNGIBLE_BALANCE">CONCURRENT_FUNGIBLE_BALANCE</a>: u64 = 67;
</code></pre>



<a id="0x1_features_CRYPTOGRAPHY_ALGEBRA_NATIVES"></a>

Whether generic algebra basic operation support in <code>crypto_algebra.<b>move</b></code> are enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_CRYPTOGRAPHY_ALGEBRA_NATIVES">CRYPTOGRAPHY_ALGEBRA_NATIVES</a>: u64 = 12;
</code></pre>



<a id="0x1_features_DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE"></a>

Whether to default new Fungible Store to the concurrent variant.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE">DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE</a>: u64 = 68;
</code></pre>



<a id="0x1_features_DELEGATION_POOLS"></a>

Whether delegation pools are enabled.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_DELEGATION_POOLS">DELEGATION_POOLS</a>: u64 = 11;
</code></pre>



<a id="0x1_features_DELEGATION_POOL_ALLOWLISTING"></a>

Whether delegators allowlisting for delegation pools is supported.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_DELEGATION_POOL_ALLOWLISTING">DELEGATION_POOL_ALLOWLISTING</a>: u64 = 56;
</code></pre>



<a id="0x1_features_DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING"></a>

Whether enable paritial governance voting on delegation_pool.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING">DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING</a>: u64 = 21;
</code></pre>



<a id="0x1_features_DISPATCHABLE_FUNGIBLE_ASSET"></a>

Whether the dispatchable fungible asset standard feature is enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_DISPATCHABLE_FUNGIBLE_ASSET">DISPATCHABLE_FUNGIBLE_ASSET</a>: u64 = 63;
</code></pre>



<a id="0x1_features_EAPI_DISABLED"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_EAPI_DISABLED">EAPI_DISABLED</a>: u64 = 2;
</code></pre>



<a id="0x1_features_ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH"></a>

Whether native_public_key_validate aborts when a public key of the wrong length is given
Lifetime: ephemeral


<pre><code><b>const</b> <a href="features.md#0x1_features_ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH">ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH</a>: u64 = 14;
</code></pre>



<a id="0x1_features_EFEATURE_CANNOT_BE_DISABLED"></a>

Deployed to production, and disabling is deprecated.


<pre><code><b>const</b> <a href="features.md#0x1_features_EFEATURE_CANNOT_BE_DISABLED">EFEATURE_CANNOT_BE_DISABLED</a>: u64 = 3;
</code></pre>



<a id="0x1_features_EFRAMEWORK_SIGNER_NEEDED"></a>

The provided signer has not a framework address.


<pre><code><b>const</b> <a href="features.md#0x1_features_EFRAMEWORK_SIGNER_NEEDED">EFRAMEWORK_SIGNER_NEEDED</a>: u64 = 1;
</code></pre>



<a id="0x1_features_EINVALID_FEATURE"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_EINVALID_FEATURE">EINVALID_FEATURE</a>: u64 = 1;
</code></pre>



<a id="0x1_features_FEE_PAYER_ACCOUNT_OPTIONAL"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_FEE_PAYER_ACCOUNT_OPTIONAL">FEE_PAYER_ACCOUNT_OPTIONAL</a>: u64 = 35;
</code></pre>



<a id="0x1_features_FEE_PAYER_ENABLED"></a>

Whether alternate gas payer is supported
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_FEE_PAYER_ENABLED">FEE_PAYER_ENABLED</a>: u64 = 22;
</code></pre>



<a id="0x1_features_JWK_CONSENSUS"></a>

Deprecated by <code>aptos_framework::jwk_consensus_config::JWKConsensusConfig</code>.


<pre><code><b>const</b> <a href="features.md#0x1_features_JWK_CONSENSUS">JWK_CONSENSUS</a>: u64 = 49;
</code></pre>



<a id="0x1_features_KEYLESS_ACCOUNTS"></a>

Whether the OIDB feature is enabled, possibly with the ZK-less verification mode.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_KEYLESS_ACCOUNTS">KEYLESS_ACCOUNTS</a>: u64 = 46;
</code></pre>



<a id="0x1_features_KEYLESS_ACCOUNTS_WITH_PASSKEYS"></a>

Whether keyless accounts support passkey-based ephemeral signatures.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_KEYLESS_ACCOUNTS_WITH_PASSKEYS">KEYLESS_ACCOUNTS_WITH_PASSKEYS</a>: u64 = 54;
</code></pre>



<a id="0x1_features_KEYLESS_BUT_ZKLESS_ACCOUNTS"></a>

Whether the ZK-less mode of the keyless accounts feature is enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_KEYLESS_BUT_ZKLESS_ACCOUNTS">KEYLESS_BUT_ZKLESS_ACCOUNTS</a>: u64 = 47;
</code></pre>



<a id="0x1_features_LIMIT_MAX_IDENTIFIER_LENGTH"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_LIMIT_MAX_IDENTIFIER_LENGTH">LIMIT_MAX_IDENTIFIER_LENGTH</a>: u64 = 38;
</code></pre>



<a id="0x1_features_MAX_OBJECT_NESTING_CHECK"></a>

Whether checking the maximum object nesting is enabled.


<pre><code><b>const</b> <a href="features.md#0x1_features_MAX_OBJECT_NESTING_CHECK">MAX_OBJECT_NESTING_CHECK</a>: u64 = 53;
</code></pre>



<a id="0x1_features_MODULE_EVENT"></a>

Whether emit function in <code>event.<b>move</b></code> are enabled for module events.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_MODULE_EVENT">MODULE_EVENT</a>: u64 = 26;
</code></pre>



<a id="0x1_features_MODULE_EVENT_MIGRATION"></a>

Whether aptos_framwork enables the behavior of module event migration.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_MODULE_EVENT_MIGRATION">MODULE_EVENT_MIGRATION</a>: u64 = 57;
</code></pre>



<a id="0x1_features_MULTISIG_ACCOUNTS"></a>

Whether multisig accounts (different from accounts with multi-ed25519 auth keys) are enabled.


<pre><code><b>const</b> <a href="features.md#0x1_features_MULTISIG_ACCOUNTS">MULTISIG_ACCOUNTS</a>: u64 = 10;
</code></pre>



<a id="0x1_features_MULTISIG_V2_ENHANCEMENT"></a>

Whether the Multisig V2 enhancement feature is enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_MULTISIG_V2_ENHANCEMENT">MULTISIG_V2_ENHANCEMENT</a>: u64 = 55;
</code></pre>



<a id="0x1_features_MULTI_ED25519_PK_VALIDATE_V2_NATIVES"></a>

Whether the new <code>aptos_stdlib::multi_ed25519::public_key_validate_internal_v2()</code> native is enabled.
This is needed because of the introduction of a new native function.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_MULTI_ED25519_PK_VALIDATE_V2_NATIVES">MULTI_ED25519_PK_VALIDATE_V2_NATIVES</a>: u64 = 7;
</code></pre>



<a id="0x1_features_NEW_ACCOUNTS_DEFAULT_TO_FA_SUPRA_STORE"></a>

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_NEW_ACCOUNTS_DEFAULT_TO_FA_SUPRA_STORE">NEW_ACCOUNTS_DEFAULT_TO_FA_SUPRA_STORE</a>: u64 = 64;
</code></pre>



<a id="0x1_features_OBJECT_CODE_DEPLOYMENT"></a>

Whether deploying to objects is enabled.


<pre><code><b>const</b> <a href="features.md#0x1_features_OBJECT_CODE_DEPLOYMENT">OBJECT_CODE_DEPLOYMENT</a>: u64 = 52;
</code></pre>



<a id="0x1_features_OBJECT_NATIVE_DERIVED_ADDRESS"></a>

Whether we use more efficient native implementation of computing object derived address


<pre><code><b>const</b> <a href="features.md#0x1_features_OBJECT_NATIVE_DERIVED_ADDRESS">OBJECT_NATIVE_DERIVED_ADDRESS</a>: u64 = 62;
</code></pre>



<a id="0x1_features_OPERATIONS_DEFAULT_TO_FA_SUPRA_STORE"></a>

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_OPERATIONS_DEFAULT_TO_FA_SUPRA_STORE">OPERATIONS_DEFAULT_TO_FA_SUPRA_STORE</a>: u64 = 65;
</code></pre>



<a id="0x1_features_OPERATOR_BENEFICIARY_CHANGE"></a>

Whether allow changing beneficiaries for operators.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_OPERATOR_BENEFICIARY_CHANGE">OPERATOR_BENEFICIARY_CHANGE</a>: u64 = 39;
</code></pre>



<a id="0x1_features_PARTIAL_GOVERNANCE_VOTING"></a>

Whether enable paritial governance voting on supra_governance.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_PARTIAL_GOVERNANCE_VOTING">PARTIAL_GOVERNANCE_VOTING</a>: u64 = 17;
</code></pre>



<a id="0x1_features_PERIODICAL_REWARD_RATE_DECREASE"></a>

Whether reward rate decreases periodically.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_PERIODICAL_REWARD_RATE_DECREASE">PERIODICAL_REWARD_RATE_DECREASE</a>: u64 = 16;
</code></pre>



<a id="0x1_features_PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS">PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS</a>: u64 = 61;
</code></pre>



<a id="0x1_features_PRIVATE_POLL"></a>

Whether the APIs related to the bls12381_bulletproofs feature are enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_PRIVATE_POLL">PRIVATE_POLL</a>: u64 = 91;
</code></pre>



<a id="0x1_features_RECONFIGURE_WITH_DKG"></a>

Deprecated by <code>aptos_framework::randomness_config::RandomnessConfig</code>.


<pre><code><b>const</b> <a href="features.md#0x1_features_RECONFIGURE_WITH_DKG">RECONFIGURE_WITH_DKG</a>: u64 = 45;
</code></pre>



<a id="0x1_features_RESOURCE_GROUPS"></a>

Whether resource groups are enabled.
This is needed because of new attributes for structs and a change in storage representation.


<pre><code><b>const</b> <a href="features.md#0x1_features_RESOURCE_GROUPS">RESOURCE_GROUPS</a>: u64 = 9;
</code></pre>



<a id="0x1_features_RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET">RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET</a>: u64 = 41;
</code></pre>



<a id="0x1_features_SAFER_METADATA"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_SAFER_METADATA">SAFER_METADATA</a>: u64 = 32;
</code></pre>



<a id="0x1_features_SAFER_RESOURCE_GROUPS"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_SAFER_RESOURCE_GROUPS">SAFER_RESOURCE_GROUPS</a>: u64 = 31;
</code></pre>



<a id="0x1_features_SHA_512_AND_RIPEMD_160_NATIVES"></a>

Whether the new SHA2-512, SHA3-512 and RIPEMD-160 hash function natives are enabled.
This is needed because of the introduction of new native functions.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SHA_512_AND_RIPEMD_160_NATIVES">SHA_512_AND_RIPEMD_160_NATIVES</a>: u64 = 3;
</code></pre>



<a id="0x1_features_SIGNATURE_CHECKER_V2_SCRIPT_FIX"></a>

Whether the fix for a counting bug in the script path of the signature checker pass is enabled.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SIGNATURE_CHECKER_V2_SCRIPT_FIX">SIGNATURE_CHECKER_V2_SCRIPT_FIX</a>: u64 = 29;
</code></pre>



<a id="0x1_features_SIGNER_NATIVE_FORMAT_FIX"></a>

Fix the native formatter for signer.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SIGNER_NATIVE_FORMAT_FIX">SIGNER_NATIVE_FORMAT_FIX</a>: u64 = 25;
</code></pre>



<a id="0x1_features_SINGLE_SENDER_AUTHENTICATOR"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_SINGLE_SENDER_AUTHENTICATOR">SINGLE_SENDER_AUTHENTICATOR</a>: u64 = 33;
</code></pre>



<a id="0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION"></a>

Whether the automatic creation of accounts is enabled for sponsored transactions.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>: u64 = 34;
</code></pre>



<a id="0x1_features_STRUCT_CONSTRUCTORS"></a>

Whether struct constructors are enabled

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_STRUCT_CONSTRUCTORS">STRUCT_CONSTRUCTORS</a>: u64 = 15;
</code></pre>



<a id="0x1_features_SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK"></a>

Whether gas check of automation-task during registration is enabled. Once enabled, the inner payload along with
task gas parameters in scope of automation registration transaction will pass gas check.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK">SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK</a>: u64 = 90;
</code></pre>



<a id="0x1_features_SUPRA_AUTOMATION_TASK_SYNC"></a>

Whether the automation task sync on block basis is enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SUPRA_AUTOMATION_TASK_SYNC">SUPRA_AUTOMATION_TASK_SYNC</a>: u64 = 92;
</code></pre>



<a id="0x1_features_SUPRA_COUNT_FAILED_PROPOSALS"></a>

Whether the automation task sync on block basis is enabled.


<pre><code><b>const</b> <a href="features.md#0x1_features_SUPRA_COUNT_FAILED_PROPOSALS">SUPRA_COUNT_FAILED_PROPOSALS</a>: u64 = 93;
</code></pre>



<a id="0x1_features_SUPRA_ETH_TRIE"></a>

Whether the APIs related to the eth_trie feature are enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SUPRA_ETH_TRIE">SUPRA_ETH_TRIE</a>: u64 = 89;
</code></pre>



<a id="0x1_features_SUPRA_NATIVE_AUTOMATION"></a>

Whether Supra native automation is enabled. Once enabled, the native automation feature can be utilized,
enabling automation task registration, execution and cancellation.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SUPRA_NATIVE_AUTOMATION">SUPRA_NATIVE_AUTOMATION</a>: u64 = 88;
</code></pre>



<a id="0x1_features_SUPRA_RLP_ENCODE"></a>

Whether the APIs related to the rlp feature are enabled.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_SUPRA_RLP_ENCODE">SUPRA_RLP_ENCODE</a>: u64 = 94;
</code></pre>



<a id="0x1_features_TRANSACTION_CONTEXT_EXTENSION"></a>

Whether the transaction context extension is enabled. This feature allows the module
<code>transaction_context</code> to provide contextual information about the user transaction.

Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_TRANSACTION_CONTEXT_EXTENSION">TRANSACTION_CONTEXT_EXTENSION</a>: u64 = 59;
</code></pre>



<a id="0x1_features_TREAT_FRIEND_AS_PRIVATE"></a>

Whether during upgrade compatibility checking, friend functions should be treated similar like
private functions.
Lifetime: permanent


<pre><code><b>const</b> <a href="features.md#0x1_features_TREAT_FRIEND_AS_PRIVATE">TREAT_FRIEND_AS_PRIVATE</a>: u64 = 2;
</code></pre>



<a id="0x1_features_VM_BINARY_FORMAT_V6"></a>

Whether to allow the use of binary format version v6.
Lifetime: transient


<pre><code><b>const</b> <a href="features.md#0x1_features_VM_BINARY_FORMAT_V6">VM_BINARY_FORMAT_V6</a>: u64 = 5;
</code></pre>



<a id="0x1_features_VM_BINARY_FORMAT_V7"></a>



<pre><code><b>const</b> <a href="features.md#0x1_features_VM_BINARY_FORMAT_V7">VM_BINARY_FORMAT_V7</a>: u64 = 40;
</code></pre>



<a id="0x1_features_code_dependency_check_enabled"></a>

## Function `code_dependency_check_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_code_dependency_check_enabled">code_dependency_check_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_code_dependency_check_enabled">code_dependency_check_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_CODE_DEPENDENCY_CHECK">CODE_DEPENDENCY_CHECK</a>)
}
</code></pre>



</details>

<a id="0x1_features_treat_friend_as_private"></a>

## Function `treat_friend_as_private`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_treat_friend_as_private">treat_friend_as_private</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_treat_friend_as_private">treat_friend_as_private</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_TREAT_FRIEND_AS_PRIVATE">TREAT_FRIEND_AS_PRIVATE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_sha_512_and_ripemd_160_feature"></a>

## Function `get_sha_512_and_ripemd_160_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_sha_512_and_ripemd_160_feature">get_sha_512_and_ripemd_160_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_sha_512_and_ripemd_160_feature">get_sha_512_and_ripemd_160_feature</a>(): u64 {
    <a href="features.md#0x1_features_SHA_512_AND_RIPEMD_160_NATIVES">SHA_512_AND_RIPEMD_160_NATIVES</a>
}
</code></pre>



</details>

<a id="0x1_features_sha_512_and_ripemd_160_enabled"></a>

## Function `sha_512_and_ripemd_160_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_sha_512_and_ripemd_160_enabled">sha_512_and_ripemd_160_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_sha_512_and_ripemd_160_enabled">sha_512_and_ripemd_160_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SHA_512_AND_RIPEMD_160_NATIVES">SHA_512_AND_RIPEMD_160_NATIVES</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_aptos_stdlib_chain_id_feature"></a>

## Function `get_aptos_stdlib_chain_id_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_aptos_stdlib_chain_id_feature">get_aptos_stdlib_chain_id_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_aptos_stdlib_chain_id_feature">get_aptos_stdlib_chain_id_feature</a>(): u64 {
    <a href="features.md#0x1_features_APTOS_STD_CHAIN_ID_NATIVES">APTOS_STD_CHAIN_ID_NATIVES</a>
}
</code></pre>



</details>

<a id="0x1_features_aptos_stdlib_chain_id_enabled"></a>

## Function `aptos_stdlib_chain_id_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_aptos_stdlib_chain_id_enabled">aptos_stdlib_chain_id_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_aptos_stdlib_chain_id_enabled">aptos_stdlib_chain_id_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_APTOS_STD_CHAIN_ID_NATIVES">APTOS_STD_CHAIN_ID_NATIVES</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_vm_binary_format_v6"></a>

## Function `get_vm_binary_format_v6`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_vm_binary_format_v6">get_vm_binary_format_v6</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_vm_binary_format_v6">get_vm_binary_format_v6</a>(): u64 {
    <a href="features.md#0x1_features_VM_BINARY_FORMAT_V6">VM_BINARY_FORMAT_V6</a>
}
</code></pre>



</details>

<a id="0x1_features_allow_vm_binary_format_v6"></a>

## Function `allow_vm_binary_format_v6`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_allow_vm_binary_format_v6">allow_vm_binary_format_v6</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_allow_vm_binary_format_v6">allow_vm_binary_format_v6</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_VM_BINARY_FORMAT_V6">VM_BINARY_FORMAT_V6</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_collect_and_distribute_gas_fees_feature"></a>

## Function `get_collect_and_distribute_gas_fees_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_collect_and_distribute_gas_fees_feature">get_collect_and_distribute_gas_fees_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_collect_and_distribute_gas_fees_feature">get_collect_and_distribute_gas_fees_feature</a>(): u64 {
    <a href="features.md#0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES">COLLECT_AND_DISTRIBUTE_GAS_FEES</a>
}
</code></pre>



</details>

<a id="0x1_features_collect_and_distribute_gas_fees"></a>

## Function `collect_and_distribute_gas_fees`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_collect_and_distribute_gas_fees">collect_and_distribute_gas_fees</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_collect_and_distribute_gas_fees">collect_and_distribute_gas_fees</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES">COLLECT_AND_DISTRIBUTE_GAS_FEES</a>)
}
</code></pre>



</details>

<a id="0x1_features_multi_ed25519_pk_validate_v2_feature"></a>

## Function `multi_ed25519_pk_validate_v2_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multi_ed25519_pk_validate_v2_feature">multi_ed25519_pk_validate_v2_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multi_ed25519_pk_validate_v2_feature">multi_ed25519_pk_validate_v2_feature</a>(): u64 {
    <a href="features.md#0x1_features_MULTI_ED25519_PK_VALIDATE_V2_NATIVES">MULTI_ED25519_PK_VALIDATE_V2_NATIVES</a>
}
</code></pre>



</details>

<a id="0x1_features_multi_ed25519_pk_validate_v2_enabled"></a>

## Function `multi_ed25519_pk_validate_v2_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multi_ed25519_pk_validate_v2_enabled">multi_ed25519_pk_validate_v2_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multi_ed25519_pk_validate_v2_enabled">multi_ed25519_pk_validate_v2_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_MULTI_ED25519_PK_VALIDATE_V2_NATIVES">MULTI_ED25519_PK_VALIDATE_V2_NATIVES</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_blake2b_256_feature"></a>

## Function `get_blake2b_256_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_blake2b_256_feature">get_blake2b_256_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_blake2b_256_feature">get_blake2b_256_feature</a>(): u64 {
    <a href="features.md#0x1_features_BLAKE2B_256_NATIVE">BLAKE2B_256_NATIVE</a>
}
</code></pre>



</details>

<a id="0x1_features_blake2b_256_enabled"></a>

## Function `blake2b_256_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_blake2b_256_enabled">blake2b_256_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_blake2b_256_enabled">blake2b_256_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_BLAKE2B_256_NATIVE">BLAKE2B_256_NATIVE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_resource_groups_feature"></a>

## Function `get_resource_groups_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_resource_groups_feature">get_resource_groups_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_resource_groups_feature">get_resource_groups_feature</a>(): u64 {
    <a href="features.md#0x1_features_RESOURCE_GROUPS">RESOURCE_GROUPS</a>
}
</code></pre>



</details>

<a id="0x1_features_resource_groups_enabled"></a>

## Function `resource_groups_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_resource_groups_enabled">resource_groups_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_resource_groups_enabled">resource_groups_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_RESOURCE_GROUPS">RESOURCE_GROUPS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_multisig_accounts_feature"></a>

## Function `get_multisig_accounts_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_multisig_accounts_feature">get_multisig_accounts_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_multisig_accounts_feature">get_multisig_accounts_feature</a>(): u64 {
    <a href="features.md#0x1_features_MULTISIG_ACCOUNTS">MULTISIG_ACCOUNTS</a>
}
</code></pre>



</details>

<a id="0x1_features_multisig_accounts_enabled"></a>

## Function `multisig_accounts_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multisig_accounts_enabled">multisig_accounts_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multisig_accounts_enabled">multisig_accounts_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_MULTISIG_ACCOUNTS">MULTISIG_ACCOUNTS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_delegation_pools_feature"></a>

## Function `get_delegation_pools_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_delegation_pools_feature">get_delegation_pools_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_delegation_pools_feature">get_delegation_pools_feature</a>(): u64 {
    <a href="features.md#0x1_features_DELEGATION_POOLS">DELEGATION_POOLS</a>
}
</code></pre>



</details>

<a id="0x1_features_delegation_pools_enabled"></a>

## Function `delegation_pools_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_delegation_pools_enabled">delegation_pools_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_delegation_pools_enabled">delegation_pools_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_DELEGATION_POOLS">DELEGATION_POOLS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_cryptography_algebra_natives_feature"></a>

## Function `get_cryptography_algebra_natives_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_cryptography_algebra_natives_feature">get_cryptography_algebra_natives_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_cryptography_algebra_natives_feature">get_cryptography_algebra_natives_feature</a>(): u64 {
    <a href="features.md#0x1_features_CRYPTOGRAPHY_ALGEBRA_NATIVES">CRYPTOGRAPHY_ALGEBRA_NATIVES</a>
}
</code></pre>



</details>

<a id="0x1_features_cryptography_algebra_enabled"></a>

## Function `cryptography_algebra_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_cryptography_algebra_enabled">cryptography_algebra_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_cryptography_algebra_enabled">cryptography_algebra_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_CRYPTOGRAPHY_ALGEBRA_NATIVES">CRYPTOGRAPHY_ALGEBRA_NATIVES</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_bls12_381_strutures_feature"></a>

## Function `get_bls12_381_strutures_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_bls12_381_strutures_feature">get_bls12_381_strutures_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_bls12_381_strutures_feature">get_bls12_381_strutures_feature</a>(): u64 {
    <a href="features.md#0x1_features_BLS12_381_STRUCTURES">BLS12_381_STRUCTURES</a>
}
</code></pre>



</details>

<a id="0x1_features_bls12_381_structures_enabled"></a>

## Function `bls12_381_structures_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_bls12_381_structures_enabled">bls12_381_structures_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_bls12_381_structures_enabled">bls12_381_structures_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_BLS12_381_STRUCTURES">BLS12_381_STRUCTURES</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_periodical_reward_rate_decrease_feature"></a>

## Function `get_periodical_reward_rate_decrease_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_periodical_reward_rate_decrease_feature">get_periodical_reward_rate_decrease_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_periodical_reward_rate_decrease_feature">get_periodical_reward_rate_decrease_feature</a>(): u64 {
    <a href="features.md#0x1_features_PERIODICAL_REWARD_RATE_DECREASE">PERIODICAL_REWARD_RATE_DECREASE</a>
}
</code></pre>



</details>

<a id="0x1_features_periodical_reward_rate_decrease_enabled"></a>

## Function `periodical_reward_rate_decrease_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_periodical_reward_rate_decrease_enabled">periodical_reward_rate_decrease_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_periodical_reward_rate_decrease_enabled">periodical_reward_rate_decrease_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_PERIODICAL_REWARD_RATE_DECREASE">PERIODICAL_REWARD_RATE_DECREASE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_partial_governance_voting"></a>

## Function `get_partial_governance_voting`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_partial_governance_voting">get_partial_governance_voting</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_partial_governance_voting">get_partial_governance_voting</a>(): u64 {
    <a href="features.md#0x1_features_PARTIAL_GOVERNANCE_VOTING">PARTIAL_GOVERNANCE_VOTING</a>
}
</code></pre>



</details>

<a id="0x1_features_partial_governance_voting_enabled"></a>

## Function `partial_governance_voting_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_PARTIAL_GOVERNANCE_VOTING">PARTIAL_GOVERNANCE_VOTING</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_delegation_pool_partial_governance_voting"></a>

## Function `get_delegation_pool_partial_governance_voting`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_delegation_pool_partial_governance_voting">get_delegation_pool_partial_governance_voting</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_delegation_pool_partial_governance_voting">get_delegation_pool_partial_governance_voting</a>(): u64 {
    <a href="features.md#0x1_features_DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING">DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING</a>
}
</code></pre>



</details>

<a id="0x1_features_delegation_pool_partial_governance_voting_enabled"></a>

## Function `delegation_pool_partial_governance_voting_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_delegation_pool_partial_governance_voting_enabled">delegation_pool_partial_governance_voting_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_delegation_pool_partial_governance_voting_enabled">delegation_pool_partial_governance_voting_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING">DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING</a>)
}
</code></pre>



</details>

<a id="0x1_features_fee_payer_enabled"></a>

## Function `fee_payer_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_fee_payer_enabled">fee_payer_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_fee_payer_enabled">fee_payer_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_FEE_PAYER_ENABLED">FEE_PAYER_ENABLED</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_auids"></a>

## Function `get_auids`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_auids">get_auids</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_auids">get_auids</a>(): u64 {
    <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EFEATURE_CANNOT_BE_DISABLED">EFEATURE_CANNOT_BE_DISABLED</a>)
}
</code></pre>



</details>

<a id="0x1_features_auids_enabled"></a>

## Function `auids_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_auids_enabled">auids_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_auids_enabled">auids_enabled</a>(): bool {
    <b>true</b>
}
</code></pre>



</details>

<a id="0x1_features_get_bulletproofs_feature"></a>

## Function `get_bulletproofs_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_bulletproofs_feature">get_bulletproofs_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_bulletproofs_feature">get_bulletproofs_feature</a>(): u64 {
    <a href="features.md#0x1_features_BULLETPROOFS_NATIVES">BULLETPROOFS_NATIVES</a>
}
</code></pre>



</details>

<a id="0x1_features_bulletproofs_enabled"></a>

## Function `bulletproofs_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_bulletproofs_enabled">bulletproofs_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_bulletproofs_enabled">bulletproofs_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_BULLETPROOFS_NATIVES">BULLETPROOFS_NATIVES</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_signer_native_format_fix_feature"></a>

## Function `get_signer_native_format_fix_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_signer_native_format_fix_feature">get_signer_native_format_fix_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_signer_native_format_fix_feature">get_signer_native_format_fix_feature</a>(): u64 {
    <a href="features.md#0x1_features_SIGNER_NATIVE_FORMAT_FIX">SIGNER_NATIVE_FORMAT_FIX</a>
}
</code></pre>



</details>

<a id="0x1_features_signer_native_format_fix_enabled"></a>

## Function `signer_native_format_fix_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_signer_native_format_fix_enabled">signer_native_format_fix_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_signer_native_format_fix_enabled">signer_native_format_fix_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SIGNER_NATIVE_FORMAT_FIX">SIGNER_NATIVE_FORMAT_FIX</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_module_event_feature"></a>

## Function `get_module_event_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_module_event_feature">get_module_event_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_module_event_feature">get_module_event_feature</a>(): u64 {
    <a href="features.md#0x1_features_MODULE_EVENT">MODULE_EVENT</a>
}
</code></pre>



</details>

<a id="0x1_features_module_event_enabled"></a>

## Function `module_event_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_module_event_enabled">module_event_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_module_event_enabled">module_event_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_MODULE_EVENT">MODULE_EVENT</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_aggregator_v2_api_feature"></a>

## Function `get_aggregator_v2_api_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_aggregator_v2_api_feature">get_aggregator_v2_api_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_aggregator_v2_api_feature">get_aggregator_v2_api_feature</a>(): u64 {
    <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EFEATURE_CANNOT_BE_DISABLED">EFEATURE_CANNOT_BE_DISABLED</a>)
}
</code></pre>



</details>

<a id="0x1_features_aggregator_v2_api_enabled"></a>

## Function `aggregator_v2_api_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_aggregator_v2_api_enabled">aggregator_v2_api_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_aggregator_v2_api_enabled">aggregator_v2_api_enabled</a>(): bool {
    <b>true</b>
}
</code></pre>



</details>

<a id="0x1_features_get_aggregator_snapshots_feature"></a>

## Function `get_aggregator_snapshots_feature`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_aggregator_snapshots_feature">get_aggregator_snapshots_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_aggregator_snapshots_feature">get_aggregator_snapshots_feature</a>(): u64 {
    <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EINVALID_FEATURE">EINVALID_FEATURE</a>)
}
</code></pre>



</details>

<a id="0x1_features_aggregator_snapshots_enabled"></a>

## Function `aggregator_snapshots_enabled`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_aggregator_snapshots_enabled">aggregator_snapshots_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_aggregator_snapshots_enabled">aggregator_snapshots_enabled</a>(): bool {
    <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EINVALID_FEATURE">EINVALID_FEATURE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_sponsored_automatic_account_creation"></a>

## Function `get_sponsored_automatic_account_creation`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_sponsored_automatic_account_creation">get_sponsored_automatic_account_creation</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_sponsored_automatic_account_creation">get_sponsored_automatic_account_creation</a>(): u64 {
    <a href="features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>
}
</code></pre>



</details>

<a id="0x1_features_sponsored_automatic_account_creation_enabled"></a>

## Function `sponsored_automatic_account_creation_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_sponsored_automatic_account_creation_enabled">sponsored_automatic_account_creation_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_sponsored_automatic_account_creation_enabled">sponsored_automatic_account_creation_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION">SPONSORED_AUTOMATIC_ACCOUNT_CREATION</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_concurrent_token_v2_feature"></a>

## Function `get_concurrent_token_v2_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_token_v2_feature">get_concurrent_token_v2_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_token_v2_feature">get_concurrent_token_v2_feature</a>(): u64 {
    <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EFEATURE_CANNOT_BE_DISABLED">EFEATURE_CANNOT_BE_DISABLED</a>)
}
</code></pre>



</details>

<a id="0x1_features_concurrent_token_v2_enabled"></a>

## Function `concurrent_token_v2_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_token_v2_enabled">concurrent_token_v2_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_token_v2_enabled">concurrent_token_v2_enabled</a>(): bool {
    <b>true</b>
}
</code></pre>



</details>

<a id="0x1_features_get_concurrent_assets_feature"></a>

## Function `get_concurrent_assets_feature`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_assets_feature">get_concurrent_assets_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_assets_feature">get_concurrent_assets_feature</a>(): u64 {
    <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EFEATURE_CANNOT_BE_DISABLED">EFEATURE_CANNOT_BE_DISABLED</a>)
}
</code></pre>



</details>

<a id="0x1_features_concurrent_assets_enabled"></a>

## Function `concurrent_assets_enabled`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_assets_enabled">concurrent_assets_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_assets_enabled">concurrent_assets_enabled</a>(): bool {
    <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EFEATURE_CANNOT_BE_DISABLED">EFEATURE_CANNOT_BE_DISABLED</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_operator_beneficiary_change_feature"></a>

## Function `get_operator_beneficiary_change_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_operator_beneficiary_change_feature">get_operator_beneficiary_change_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_operator_beneficiary_change_feature">get_operator_beneficiary_change_feature</a>(): u64 {
    <a href="features.md#0x1_features_OPERATOR_BENEFICIARY_CHANGE">OPERATOR_BENEFICIARY_CHANGE</a>
}
</code></pre>



</details>

<a id="0x1_features_operator_beneficiary_change_enabled"></a>

## Function `operator_beneficiary_change_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_operator_beneficiary_change_enabled">operator_beneficiary_change_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_operator_beneficiary_change_enabled">operator_beneficiary_change_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_OPERATOR_BENEFICIARY_CHANGE">OPERATOR_BENEFICIARY_CHANGE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_commission_change_delegation_pool_feature"></a>

## Function `get_commission_change_delegation_pool_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_commission_change_delegation_pool_feature">get_commission_change_delegation_pool_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_commission_change_delegation_pool_feature">get_commission_change_delegation_pool_feature</a>(): u64 {
    <a href="features.md#0x1_features_COMMISSION_CHANGE_DELEGATION_POOL">COMMISSION_CHANGE_DELEGATION_POOL</a>
}
</code></pre>



</details>

<a id="0x1_features_commission_change_delegation_pool_enabled"></a>

## Function `commission_change_delegation_pool_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_commission_change_delegation_pool_enabled">commission_change_delegation_pool_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_commission_change_delegation_pool_enabled">commission_change_delegation_pool_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_COMMISSION_CHANGE_DELEGATION_POOL">COMMISSION_CHANGE_DELEGATION_POOL</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_bn254_strutures_feature"></a>

## Function `get_bn254_strutures_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_bn254_strutures_feature">get_bn254_strutures_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_bn254_strutures_feature">get_bn254_strutures_feature</a>(): u64 {
    <a href="features.md#0x1_features_BN254_STRUCTURES">BN254_STRUCTURES</a>
}
</code></pre>



</details>

<a id="0x1_features_bn254_structures_enabled"></a>

## Function `bn254_structures_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_bn254_structures_enabled">bn254_structures_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_bn254_structures_enabled">bn254_structures_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_BN254_STRUCTURES">BN254_STRUCTURES</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_reconfigure_with_dkg_feature"></a>

## Function `get_reconfigure_with_dkg_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_reconfigure_with_dkg_feature">get_reconfigure_with_dkg_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_reconfigure_with_dkg_feature">get_reconfigure_with_dkg_feature</a>(): u64 {
    <a href="features.md#0x1_features_RECONFIGURE_WITH_DKG">RECONFIGURE_WITH_DKG</a>
}
</code></pre>



</details>

<a id="0x1_features_reconfigure_with_dkg_enabled"></a>

## Function `reconfigure_with_dkg_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_reconfigure_with_dkg_enabled">reconfigure_with_dkg_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_reconfigure_with_dkg_enabled">reconfigure_with_dkg_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_RECONFIGURE_WITH_DKG">RECONFIGURE_WITH_DKG</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_keyless_accounts_feature"></a>

## Function `get_keyless_accounts_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_keyless_accounts_feature">get_keyless_accounts_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_keyless_accounts_feature">get_keyless_accounts_feature</a>(): u64 {
    <a href="features.md#0x1_features_KEYLESS_ACCOUNTS">KEYLESS_ACCOUNTS</a>
}
</code></pre>



</details>

<a id="0x1_features_keyless_accounts_enabled"></a>

## Function `keyless_accounts_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_keyless_accounts_enabled">keyless_accounts_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_keyless_accounts_enabled">keyless_accounts_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_KEYLESS_ACCOUNTS">KEYLESS_ACCOUNTS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_keyless_but_zkless_accounts_feature"></a>

## Function `get_keyless_but_zkless_accounts_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_keyless_but_zkless_accounts_feature">get_keyless_but_zkless_accounts_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_keyless_but_zkless_accounts_feature">get_keyless_but_zkless_accounts_feature</a>(): u64 {
    <a href="features.md#0x1_features_KEYLESS_BUT_ZKLESS_ACCOUNTS">KEYLESS_BUT_ZKLESS_ACCOUNTS</a>
}
</code></pre>



</details>

<a id="0x1_features_keyless_but_zkless_accounts_feature_enabled"></a>

## Function `keyless_but_zkless_accounts_feature_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_keyless_but_zkless_accounts_feature_enabled">keyless_but_zkless_accounts_feature_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_keyless_but_zkless_accounts_feature_enabled">keyless_but_zkless_accounts_feature_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_KEYLESS_BUT_ZKLESS_ACCOUNTS">KEYLESS_BUT_ZKLESS_ACCOUNTS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_jwk_consensus_feature"></a>

## Function `get_jwk_consensus_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_jwk_consensus_feature">get_jwk_consensus_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_jwk_consensus_feature">get_jwk_consensus_feature</a>(): u64 {
    <a href="features.md#0x1_features_JWK_CONSENSUS">JWK_CONSENSUS</a>
}
</code></pre>



</details>

<a id="0x1_features_jwk_consensus_enabled"></a>

## Function `jwk_consensus_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_jwk_consensus_enabled">jwk_consensus_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_jwk_consensus_enabled">jwk_consensus_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_JWK_CONSENSUS">JWK_CONSENSUS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_concurrent_fungible_assets_feature"></a>

## Function `get_concurrent_fungible_assets_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_fungible_assets_feature">get_concurrent_fungible_assets_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_fungible_assets_feature">get_concurrent_fungible_assets_feature</a>(): u64 {
    <a href="features.md#0x1_features_CONCURRENT_FUNGIBLE_ASSETS">CONCURRENT_FUNGIBLE_ASSETS</a>
}
</code></pre>



</details>

<a id="0x1_features_concurrent_fungible_assets_enabled"></a>

## Function `concurrent_fungible_assets_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_fungible_assets_enabled">concurrent_fungible_assets_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_fungible_assets_enabled">concurrent_fungible_assets_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_CONCURRENT_FUNGIBLE_ASSETS">CONCURRENT_FUNGIBLE_ASSETS</a>)
}
</code></pre>



</details>

<a id="0x1_features_is_object_code_deployment_enabled"></a>

## Function `is_object_code_deployment_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_is_object_code_deployment_enabled">is_object_code_deployment_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_is_object_code_deployment_enabled">is_object_code_deployment_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_OBJECT_CODE_DEPLOYMENT">OBJECT_CODE_DEPLOYMENT</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_max_object_nesting_check_feature"></a>

## Function `get_max_object_nesting_check_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_max_object_nesting_check_feature">get_max_object_nesting_check_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_max_object_nesting_check_feature">get_max_object_nesting_check_feature</a>(): u64 {
    <a href="features.md#0x1_features_MAX_OBJECT_NESTING_CHECK">MAX_OBJECT_NESTING_CHECK</a>
}
</code></pre>



</details>

<a id="0x1_features_max_object_nesting_check_enabled"></a>

## Function `max_object_nesting_check_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_max_object_nesting_check_enabled">max_object_nesting_check_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_max_object_nesting_check_enabled">max_object_nesting_check_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_MAX_OBJECT_NESTING_CHECK">MAX_OBJECT_NESTING_CHECK</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_keyless_accounts_with_passkeys_feature"></a>

## Function `get_keyless_accounts_with_passkeys_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_keyless_accounts_with_passkeys_feature">get_keyless_accounts_with_passkeys_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_keyless_accounts_with_passkeys_feature">get_keyless_accounts_with_passkeys_feature</a>(): u64 {
    <a href="features.md#0x1_features_KEYLESS_ACCOUNTS_WITH_PASSKEYS">KEYLESS_ACCOUNTS_WITH_PASSKEYS</a>
}
</code></pre>



</details>

<a id="0x1_features_keyless_accounts_with_passkeys_feature_enabled"></a>

## Function `keyless_accounts_with_passkeys_feature_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_keyless_accounts_with_passkeys_feature_enabled">keyless_accounts_with_passkeys_feature_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_keyless_accounts_with_passkeys_feature_enabled">keyless_accounts_with_passkeys_feature_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_KEYLESS_ACCOUNTS_WITH_PASSKEYS">KEYLESS_ACCOUNTS_WITH_PASSKEYS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_multisig_v2_enhancement_feature"></a>

## Function `get_multisig_v2_enhancement_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_multisig_v2_enhancement_feature">get_multisig_v2_enhancement_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_multisig_v2_enhancement_feature">get_multisig_v2_enhancement_feature</a>(): u64 {
    <a href="features.md#0x1_features_MULTISIG_V2_ENHANCEMENT">MULTISIG_V2_ENHANCEMENT</a>
}
</code></pre>



</details>

<a id="0x1_features_multisig_v2_enhancement_feature_enabled"></a>

## Function `multisig_v2_enhancement_feature_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multisig_v2_enhancement_feature_enabled">multisig_v2_enhancement_feature_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_multisig_v2_enhancement_feature_enabled">multisig_v2_enhancement_feature_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_MULTISIG_V2_ENHANCEMENT">MULTISIG_V2_ENHANCEMENT</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_delegation_pool_allowlisting_feature"></a>

## Function `get_delegation_pool_allowlisting_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_delegation_pool_allowlisting_feature">get_delegation_pool_allowlisting_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_delegation_pool_allowlisting_feature">get_delegation_pool_allowlisting_feature</a>(): u64 {
    <a href="features.md#0x1_features_DELEGATION_POOL_ALLOWLISTING">DELEGATION_POOL_ALLOWLISTING</a>
}
</code></pre>



</details>

<a id="0x1_features_delegation_pool_allowlisting_enabled"></a>

## Function `delegation_pool_allowlisting_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_delegation_pool_allowlisting_enabled">delegation_pool_allowlisting_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_delegation_pool_allowlisting_enabled">delegation_pool_allowlisting_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_DELEGATION_POOL_ALLOWLISTING">DELEGATION_POOL_ALLOWLISTING</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_module_event_migration_feature"></a>

## Function `get_module_event_migration_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_module_event_migration_feature">get_module_event_migration_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_module_event_migration_feature">get_module_event_migration_feature</a>(): u64 {
    <a href="features.md#0x1_features_MODULE_EVENT_MIGRATION">MODULE_EVENT_MIGRATION</a>
}
</code></pre>



</details>

<a id="0x1_features_module_event_migration_enabled"></a>

## Function `module_event_migration_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_module_event_migration_enabled">module_event_migration_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_module_event_migration_enabled">module_event_migration_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_MODULE_EVENT_MIGRATION">MODULE_EVENT_MIGRATION</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_transaction_context_extension_feature"></a>

## Function `get_transaction_context_extension_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_transaction_context_extension_feature">get_transaction_context_extension_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_transaction_context_extension_feature">get_transaction_context_extension_feature</a>(): u64 {
    <a href="features.md#0x1_features_TRANSACTION_CONTEXT_EXTENSION">TRANSACTION_CONTEXT_EXTENSION</a>
}
</code></pre>



</details>

<a id="0x1_features_transaction_context_extension_enabled"></a>

## Function `transaction_context_extension_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_transaction_context_extension_enabled">transaction_context_extension_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_transaction_context_extension_enabled">transaction_context_extension_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_TRANSACTION_CONTEXT_EXTENSION">TRANSACTION_CONTEXT_EXTENSION</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_coin_to_fungible_asset_migration_feature"></a>

## Function `get_coin_to_fungible_asset_migration_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_coin_to_fungible_asset_migration_feature">get_coin_to_fungible_asset_migration_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_coin_to_fungible_asset_migration_feature">get_coin_to_fungible_asset_migration_feature</a>(): u64 {
    <a href="features.md#0x1_features_COIN_TO_FUNGIBLE_ASSET_MIGRATION">COIN_TO_FUNGIBLE_ASSET_MIGRATION</a>
}
</code></pre>



</details>

<a id="0x1_features_coin_to_fungible_asset_migration_feature_enabled"></a>

## Function `coin_to_fungible_asset_migration_feature_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_coin_to_fungible_asset_migration_feature_enabled">coin_to_fungible_asset_migration_feature_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_coin_to_fungible_asset_migration_feature_enabled">coin_to_fungible_asset_migration_feature_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_COIN_TO_FUNGIBLE_ASSET_MIGRATION">COIN_TO_FUNGIBLE_ASSET_MIGRATION</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_primary_apt_fungible_store_at_user_address_feature"></a>

## Function `get_primary_apt_fungible_store_at_user_address_feature`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_primary_apt_fungible_store_at_user_address_feature">get_primary_apt_fungible_store_at_user_address_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_primary_apt_fungible_store_at_user_address_feature">get_primary_apt_fungible_store_at_user_address_feature</a>(): u64 {
    <b>abort</b> <a href="error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="features.md#0x1_features_EINVALID_FEATURE">EINVALID_FEATURE</a>)
}
</code></pre>



</details>

<a id="0x1_features_primary_apt_fungible_store_at_user_address_enabled"></a>

## Function `primary_apt_fungible_store_at_user_address_enabled`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_primary_apt_fungible_store_at_user_address_enabled">primary_apt_fungible_store_at_user_address_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_primary_apt_fungible_store_at_user_address_enabled">primary_apt_fungible_store_at_user_address_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS">PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS</a>)
}
</code></pre>



</details>

<a id="0x1_features_aggregator_v2_is_at_least_api_enabled"></a>

## Function `aggregator_v2_is_at_least_api_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_aggregator_v2_is_at_least_api_enabled">aggregator_v2_is_at_least_api_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_aggregator_v2_is_at_least_api_enabled">aggregator_v2_is_at_least_api_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_AGGREGATOR_V2_IS_AT_LEAST_API">AGGREGATOR_V2_IS_AT_LEAST_API</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_object_native_derived_address_feature"></a>

## Function `get_object_native_derived_address_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_object_native_derived_address_feature">get_object_native_derived_address_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_object_native_derived_address_feature">get_object_native_derived_address_feature</a>(): u64 {
    <a href="features.md#0x1_features_OBJECT_NATIVE_DERIVED_ADDRESS">OBJECT_NATIVE_DERIVED_ADDRESS</a>
}
</code></pre>



</details>

<a id="0x1_features_object_native_derived_address_enabled"></a>

## Function `object_native_derived_address_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_object_native_derived_address_enabled">object_native_derived_address_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_object_native_derived_address_enabled">object_native_derived_address_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_OBJECT_NATIVE_DERIVED_ADDRESS">OBJECT_NATIVE_DERIVED_ADDRESS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_dispatchable_fungible_asset_feature"></a>

## Function `get_dispatchable_fungible_asset_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_dispatchable_fungible_asset_feature">get_dispatchable_fungible_asset_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_dispatchable_fungible_asset_feature">get_dispatchable_fungible_asset_feature</a>(): u64 {
    <a href="features.md#0x1_features_DISPATCHABLE_FUNGIBLE_ASSET">DISPATCHABLE_FUNGIBLE_ASSET</a>
}
</code></pre>



</details>

<a id="0x1_features_dispatchable_fungible_asset_enabled"></a>

## Function `dispatchable_fungible_asset_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_dispatchable_fungible_asset_enabled">dispatchable_fungible_asset_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_dispatchable_fungible_asset_enabled">dispatchable_fungible_asset_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_DISPATCHABLE_FUNGIBLE_ASSET">DISPATCHABLE_FUNGIBLE_ASSET</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_new_accounts_default_to_fa_supra_store_feature"></a>

## Function `get_new_accounts_default_to_fa_supra_store_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_new_accounts_default_to_fa_supra_store_feature">get_new_accounts_default_to_fa_supra_store_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_new_accounts_default_to_fa_supra_store_feature">get_new_accounts_default_to_fa_supra_store_feature</a>(): u64 {
    <a href="features.md#0x1_features_NEW_ACCOUNTS_DEFAULT_TO_FA_SUPRA_STORE">NEW_ACCOUNTS_DEFAULT_TO_FA_SUPRA_STORE</a>
}
</code></pre>



</details>

<a id="0x1_features_new_accounts_default_to_fa_supra_store_enabled"></a>

## Function `new_accounts_default_to_fa_supra_store_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_new_accounts_default_to_fa_supra_store_enabled">new_accounts_default_to_fa_supra_store_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_new_accounts_default_to_fa_supra_store_enabled">new_accounts_default_to_fa_supra_store_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_NEW_ACCOUNTS_DEFAULT_TO_FA_SUPRA_STORE">NEW_ACCOUNTS_DEFAULT_TO_FA_SUPRA_STORE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_operations_default_to_fa_supra_store_feature"></a>

## Function `get_operations_default_to_fa_supra_store_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_operations_default_to_fa_supra_store_feature">get_operations_default_to_fa_supra_store_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_operations_default_to_fa_supra_store_feature">get_operations_default_to_fa_supra_store_feature</a>(): u64 {
    <a href="features.md#0x1_features_OPERATIONS_DEFAULT_TO_FA_SUPRA_STORE">OPERATIONS_DEFAULT_TO_FA_SUPRA_STORE</a>
}
</code></pre>



</details>

<a id="0x1_features_operations_default_to_fa_supra_store_enabled"></a>

## Function `operations_default_to_fa_supra_store_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_operations_default_to_fa_supra_store_enabled">operations_default_to_fa_supra_store_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_operations_default_to_fa_supra_store_enabled">operations_default_to_fa_supra_store_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_OPERATIONS_DEFAULT_TO_FA_SUPRA_STORE">OPERATIONS_DEFAULT_TO_FA_SUPRA_STORE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_concurrent_fungible_balance_feature"></a>

## Function `get_concurrent_fungible_balance_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_fungible_balance_feature">get_concurrent_fungible_balance_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_concurrent_fungible_balance_feature">get_concurrent_fungible_balance_feature</a>(): u64 {
    <a href="features.md#0x1_features_CONCURRENT_FUNGIBLE_BALANCE">CONCURRENT_FUNGIBLE_BALANCE</a>
}
</code></pre>



</details>

<a id="0x1_features_concurrent_fungible_balance_enabled"></a>

## Function `concurrent_fungible_balance_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_fungible_balance_enabled">concurrent_fungible_balance_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_concurrent_fungible_balance_enabled">concurrent_fungible_balance_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_CONCURRENT_FUNGIBLE_BALANCE">CONCURRENT_FUNGIBLE_BALANCE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_default_to_concurrent_fungible_balance_feature"></a>

## Function `get_default_to_concurrent_fungible_balance_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_default_to_concurrent_fungible_balance_feature">get_default_to_concurrent_fungible_balance_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_default_to_concurrent_fungible_balance_feature">get_default_to_concurrent_fungible_balance_feature</a>(): u64 {
    <a href="features.md#0x1_features_DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE">DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE</a>
}
</code></pre>



</details>

<a id="0x1_features_default_to_concurrent_fungible_balance_enabled"></a>

## Function `default_to_concurrent_fungible_balance_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_default_to_concurrent_fungible_balance_enabled">default_to_concurrent_fungible_balance_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_default_to_concurrent_fungible_balance_enabled">default_to_concurrent_fungible_balance_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE">DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_abort_if_multisig_payload_mismatch_feature"></a>

## Function `get_abort_if_multisig_payload_mismatch_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_abort_if_multisig_payload_mismatch_feature">get_abort_if_multisig_payload_mismatch_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_abort_if_multisig_payload_mismatch_feature">get_abort_if_multisig_payload_mismatch_feature</a>(): u64 {
    <a href="features.md#0x1_features_ABORT_IF_MULTISIG_PAYLOAD_MISMATCH">ABORT_IF_MULTISIG_PAYLOAD_MISMATCH</a>
}
</code></pre>



</details>

<a id="0x1_features_abort_if_multisig_payload_mismatch_enabled"></a>

## Function `abort_if_multisig_payload_mismatch_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_abort_if_multisig_payload_mismatch_enabled">abort_if_multisig_payload_mismatch_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_abort_if_multisig_payload_mismatch_enabled">abort_if_multisig_payload_mismatch_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_ABORT_IF_MULTISIG_PAYLOAD_MISMATCH">ABORT_IF_MULTISIG_PAYLOAD_MISMATCH</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_supra_native_automation_feature"></a>

## Function `get_supra_native_automation_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_native_automation_feature">get_supra_native_automation_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_native_automation_feature">get_supra_native_automation_feature</a>(): u64 {
    <a href="features.md#0x1_features_SUPRA_NATIVE_AUTOMATION">SUPRA_NATIVE_AUTOMATION</a>
}
</code></pre>



</details>

<a id="0x1_features_supra_native_automation_enabled"></a>

## Function `supra_native_automation_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_native_automation_enabled">supra_native_automation_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_native_automation_enabled">supra_native_automation_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SUPRA_NATIVE_AUTOMATION">SUPRA_NATIVE_AUTOMATION</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_supra_eth_trie_feature"></a>

## Function `get_supra_eth_trie_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_eth_trie_feature">get_supra_eth_trie_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_eth_trie_feature">get_supra_eth_trie_feature</a>(): u64 {
    <a href="features.md#0x1_features_SUPRA_ETH_TRIE">SUPRA_ETH_TRIE</a>
}
</code></pre>



</details>

<a id="0x1_features_supra_eth_trie_enabled"></a>

## Function `supra_eth_trie_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_eth_trie_enabled">supra_eth_trie_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_eth_trie_enabled">supra_eth_trie_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SUPRA_ETH_TRIE">SUPRA_ETH_TRIE</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_supra_automation_payload_gas_check_feature"></a>

## Function `get_supra_automation_payload_gas_check_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_automation_payload_gas_check_feature">get_supra_automation_payload_gas_check_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_automation_payload_gas_check_feature">get_supra_automation_payload_gas_check_feature</a>(): u64 {
    <a href="features.md#0x1_features_SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK">SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK</a>
}
</code></pre>



</details>

<a id="0x1_features_supra_automation_payload_gas_check_enabled"></a>

## Function `supra_automation_payload_gas_check_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_automation_payload_gas_check_enabled">supra_automation_payload_gas_check_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_automation_payload_gas_check_enabled">supra_automation_payload_gas_check_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK">SUPRA_AUTOMATION_PAYLOAD_GAS_CHECK</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_supra_private_poll_feature"></a>

## Function `get_supra_private_poll_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_private_poll_feature">get_supra_private_poll_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_private_poll_feature">get_supra_private_poll_feature</a>(): u64 {
    <a href="features.md#0x1_features_PRIVATE_POLL">PRIVATE_POLL</a>
}
</code></pre>



</details>

<a id="0x1_features_supra_private_poll_enabled"></a>

## Function `supra_private_poll_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_private_poll_enabled">supra_private_poll_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_private_poll_enabled">supra_private_poll_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_PRIVATE_POLL">PRIVATE_POLL</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_supra_automation_task_sync_feature"></a>

## Function `get_supra_automation_task_sync_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_automation_task_sync_feature">get_supra_automation_task_sync_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_automation_task_sync_feature">get_supra_automation_task_sync_feature</a>(): u64 {
    <a href="features.md#0x1_features_SUPRA_AUTOMATION_TASK_SYNC">SUPRA_AUTOMATION_TASK_SYNC</a>
}
</code></pre>



</details>

<a id="0x1_features_supra_automation_task_sync_enabled"></a>

## Function `supra_automation_task_sync_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_automation_task_sync_enabled">supra_automation_task_sync_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_automation_task_sync_enabled">supra_automation_task_sync_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SUPRA_AUTOMATION_TASK_SYNC">SUPRA_AUTOMATION_TASK_SYNC</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_supra_count_failed_proposals_feature"></a>

## Function `get_supra_count_failed_proposals_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_count_failed_proposals_feature">get_supra_count_failed_proposals_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_count_failed_proposals_feature">get_supra_count_failed_proposals_feature</a>(): u64 {
    <a href="features.md#0x1_features_SUPRA_COUNT_FAILED_PROPOSALS">SUPRA_COUNT_FAILED_PROPOSALS</a>
}
</code></pre>



</details>

<a id="0x1_features_supra_count_failed_proposals_enabled"></a>

## Function `supra_count_failed_proposals_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_count_failed_proposals_enabled">supra_count_failed_proposals_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_count_failed_proposals_enabled">supra_count_failed_proposals_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SUPRA_COUNT_FAILED_PROPOSALS">SUPRA_COUNT_FAILED_PROPOSALS</a>)
}
</code></pre>



</details>

<a id="0x1_features_get_supra_rlp_feature"></a>

## Function `get_supra_rlp_feature`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_rlp_feature">get_supra_rlp_feature</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_get_supra_rlp_feature">get_supra_rlp_feature</a>(): u64 {
    <a href="features.md#0x1_features_SUPRA_RLP_ENCODE">SUPRA_RLP_ENCODE</a>
}
</code></pre>



</details>

<a id="0x1_features_supra_rlp_enabled"></a>

## Function `supra_rlp_enabled`



<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_rlp_enabled">supra_rlp_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_supra_rlp_enabled">supra_rlp_enabled</a>(): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_is_enabled">is_enabled</a>(<a href="features.md#0x1_features_SUPRA_RLP_ENCODE">SUPRA_RLP_ENCODE</a>)
}
</code></pre>



</details>

<a id="0x1_features_change_feature_flags"></a>

## Function `change_feature_flags`

Deprecated to prevent validator set changes during DKG.

Genesis/tests should use <code><a href="features.md#0x1_features_change_feature_flags_internal">change_feature_flags_internal</a>()</code> for feature vec initialization.

Governance proposals should use <code><a href="features.md#0x1_features_change_feature_flags_for_next_epoch">change_feature_flags_for_next_epoch</a>()</code> to enable/disable features.


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_change_feature_flags">change_feature_flags</a>(_framework: &<a href="signer.md#0x1_signer">signer</a>, _enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, _disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_change_feature_flags">change_feature_flags</a>(
    _framework: &<a href="signer.md#0x1_signer">signer</a>, _enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, _disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;
) {
    <b>abort</b>(<a href="error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="features.md#0x1_features_EAPI_DISABLED">EAPI_DISABLED</a>))
}
</code></pre>



</details>

<a id="0x1_features_change_feature_flags_internal"></a>

## Function `change_feature_flags_internal`

Update feature flags directly. Only used in genesis/tests.


<pre><code><b>fun</b> <a href="features.md#0x1_features_change_feature_flags_internal">change_feature_flags_internal</a>(framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="features.md#0x1_features_change_feature_flags_internal">change_feature_flags_internal</a>(
    framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <b>assert</b>!(
        <a href="signer.md#0x1_signer_address_of">signer::address_of</a>(framework) == @std,
        <a href="error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="features.md#0x1_features_EFRAMEWORK_SIGNER_NEEDED">EFRAMEWORK_SIGNER_NEEDED</a>)
    );
    <b>if</b> (!<b>exists</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std)) {
        <b>move_to</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(framework, <a href="features.md#0x1_features_Features">Features</a> { <a href="features.md#0x1_features">features</a>: <a href="vector.md#0x1_vector">vector</a>[] })
    };
    <b>let</b> <a href="features.md#0x1_features">features</a> = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std).<a href="features.md#0x1_features">features</a>;
    <a href="vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(
        &enable,
        |feature| {
            <a href="features.md#0x1_features_set">set</a>(<a href="features.md#0x1_features">features</a>, *feature, <b>true</b>);
        }
    );
    <a href="vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(
        &disable,
        |feature| {
            <a href="features.md#0x1_features_set">set</a>(<a href="features.md#0x1_features">features</a>, *feature, <b>false</b>);
        }
    );
}
</code></pre>



</details>

<a id="0x1_features_change_feature_flags_for_next_epoch"></a>

## Function `change_feature_flags_for_next_epoch`

Enable and disable features for the next epoch.


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_change_feature_flags_for_next_epoch">change_feature_flags_for_next_epoch</a>(framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_change_feature_flags_for_next_epoch">change_feature_flags_for_next_epoch</a>(
    framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>, <a href="features.md#0x1_features_Features">Features</a> {
    <b>assert</b>!(
        <a href="signer.md#0x1_signer_address_of">signer::address_of</a>(framework) == @std,
        <a href="error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="features.md#0x1_features_EFRAMEWORK_SIGNER_NEEDED">EFRAMEWORK_SIGNER_NEEDED</a>)
    );

    // Figure out the baseline feature vec that the diff will be applied <b>to</b>.
    <b>let</b> new_feature_vec =
        <b>if</b> (<b>exists</b>&lt;<a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>&gt;(@std)) {
            // If there is a buffered feature vec, <b>use</b> it <b>as</b> the baseline.
            <b>let</b> <a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a> { <a href="features.md#0x1_features">features</a> } = <b>move_from</b>&lt;<a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>&gt;(@std);
            <a href="features.md#0x1_features">features</a>
        } <b>else</b> <b>if</b> (<b>exists</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std)) {
            // Otherwise, <b>use</b> the currently effective feature flag vec <b>as</b> the baseline, <b>if</b> it <b>exists</b>.
            <b>borrow_global</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std).<a href="features.md#0x1_features">features</a>
        } <b>else</b> {
            // Otherwise, <b>use</b> an empty feature vec.
            <a href="vector.md#0x1_vector">vector</a>[]
        };

    // Apply the diff and save it <b>to</b> the buffer.
    <a href="features.md#0x1_features_apply_diff">apply_diff</a>(&<b>mut</b> new_feature_vec, enable, disable);
    <b>move_to</b>(framework, <a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a> { <a href="features.md#0x1_features">features</a>: new_feature_vec });
}
</code></pre>



</details>

<a id="0x1_features_on_new_epoch"></a>

## Function `on_new_epoch`

Apply all the pending feature flag changes. Should only be used at the end of a reconfiguration with DKG.

While the scope is public, it can only be usd in system transactions like <code>block_prologue</code> and governance proposals,
who have permission to set the flag that's checked in <code>extract()</code>.


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_on_new_epoch">on_new_epoch</a>(framework: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_on_new_epoch">on_new_epoch</a>(framework: &<a href="signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a>, <a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a> {
    <a href="features.md#0x1_features_ensure_framework_signer">ensure_framework_signer</a>(framework);
    <b>if</b> (<b>exists</b>&lt;<a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>&gt;(@std)) {
        <b>let</b> <a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a> { <a href="features.md#0x1_features">features</a> } = <b>move_from</b>&lt;<a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>&gt;(@std);
        <b>if</b> (<b>exists</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std)) {
            <b>borrow_global_mut</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std).<a href="features.md#0x1_features">features</a> = <a href="features.md#0x1_features">features</a>;
        } <b>else</b> {
            <b>move_to</b>(framework, <a href="features.md#0x1_features_Features">Features</a> { <a href="features.md#0x1_features">features</a> })
        }
    }
}
</code></pre>



</details>

<a id="0x1_features_is_enabled"></a>

## Function `is_enabled`

Check whether the feature is enabled.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_is_enabled">is_enabled</a>(feature: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_is_enabled">is_enabled</a>(feature: u64): bool <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <b>exists</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std)
        && <a href="features.md#0x1_features_contains">contains</a>(&<b>borrow_global</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std).<a href="features.md#0x1_features">features</a>, feature)
}
</code></pre>



</details>

<a id="0x1_features_set"></a>

## Function `set`

Helper to include or exclude a feature flag.


<pre><code><b>fun</b> <a href="features.md#0x1_features_set">set</a>(<a href="features.md#0x1_features">features</a>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, feature: u64, <b>include</b>: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="features.md#0x1_features_set">set</a>(<a href="features.md#0x1_features">features</a>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, feature: u64, <b>include</b>: bool) {
    <b>let</b> byte_index = feature / 8;
    <b>let</b> bit_mask = 1 &lt;&lt; ((feature % 8) <b>as</b> u8);
    <b>while</b> (<a href="vector.md#0x1_vector_length">vector::length</a>(<a href="features.md#0x1_features">features</a>) &lt;= byte_index) { <a href="vector.md#0x1_vector_push_back">vector::push_back</a>(<a href="features.md#0x1_features">features</a>, 0) };
    <b>let</b> entry = <a href="vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(<a href="features.md#0x1_features">features</a>, byte_index);
    <b>if</b> (<b>include</b>) *entry = *entry | bit_mask
    <b>else</b> *entry = *entry & (0xff ^ bit_mask)
}
</code></pre>



</details>

<a id="0x1_features_contains"></a>

## Function `contains`

Helper to check whether a feature flag is enabled.


<pre><code><b>fun</b> <a href="features.md#0x1_features_contains">contains</a>(<a href="features.md#0x1_features">features</a>: &<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, feature: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="features.md#0x1_features_contains">contains</a>(<a href="features.md#0x1_features">features</a>: &<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, feature: u64): bool {
    <b>let</b> byte_index = feature / 8;
    <b>let</b> bit_mask = 1 &lt;&lt; ((feature % 8) <b>as</b> u8);
    byte_index &lt; <a href="vector.md#0x1_vector_length">vector::length</a>(<a href="features.md#0x1_features">features</a>)
        && (*<a href="vector.md#0x1_vector_borrow">vector::borrow</a>(<a href="features.md#0x1_features">features</a>, byte_index) & bit_mask) != 0
}
</code></pre>



</details>

<a id="0x1_features_apply_diff"></a>

## Function `apply_diff`



<pre><code><b>fun</b> <a href="features.md#0x1_features_apply_diff">apply_diff</a>(<a href="features.md#0x1_features">features</a>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="features.md#0x1_features_apply_diff">apply_diff</a>(
    <a href="features.md#0x1_features">features</a>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;
) {
    <a href="vector.md#0x1_vector_for_each">vector::for_each</a>(
        enable,
        |feature| {
            <a href="features.md#0x1_features_set">set</a>(<a href="features.md#0x1_features">features</a>, feature, <b>true</b>);
        }
    );
    <a href="vector.md#0x1_vector_for_each">vector::for_each</a>(
        disable,
        |feature| {
            <a href="features.md#0x1_features_set">set</a>(<a href="features.md#0x1_features">features</a>, feature, <b>false</b>);
        }
    );
}
</code></pre>



</details>

<a id="0x1_features_ensure_framework_signer"></a>

## Function `ensure_framework_signer`



<pre><code><b>fun</b> <a href="features.md#0x1_features_ensure_framework_signer">ensure_framework_signer</a>(account: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="features.md#0x1_features_ensure_framework_signer">ensure_framework_signer</a>(account: &<a href="signer.md#0x1_signer">signer</a>) {
    <b>let</b> addr = <a href="signer.md#0x1_signer_address_of">signer::address_of</a>(account);
    <b>assert</b>!(addr == @std, <a href="error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="features.md#0x1_features_EFRAMEWORK_SIGNER_NEEDED">EFRAMEWORK_SIGNER_NEEDED</a>));
}
</code></pre>



</details>

<a id="0x1_features_change_feature_flags_for_verification"></a>

## Function `change_feature_flags_for_verification`



<pre><code>#[verify_only]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_change_feature_flags_for_verification">change_feature_flags_for_verification</a>(framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_change_feature_flags_for_verification">change_feature_flags_for_verification</a>(
    framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="features.md#0x1_features_Features">Features</a> {
    <a href="features.md#0x1_features_change_feature_flags_internal">change_feature_flags_internal</a>(framework, enable, disable)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Features"></a>

### Resource `Features`


<pre><code><b>struct</b> <a href="features.md#0x1_features_Features">Features</a> <b>has</b> key
</code></pre>



<dl>
<dt>
<code><a href="features.md#0x1_features">features</a>: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>pragma</b> bv=b"0";
</code></pre>



<a id="@Specification_1_PendingFeatures"></a>

### Resource `PendingFeatures`


<pre><code><b>struct</b> <a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a> <b>has</b> key
</code></pre>



<dl>
<dt>
<code><a href="features.md#0x1_features">features</a>: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code><b>pragma</b> bv=b"0";
</code></pre>



<a id="@Specification_1_periodical_reward_rate_decrease_enabled"></a>

### Function `periodical_reward_rate_decrease_enabled`


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_periodical_reward_rate_decrease_enabled">periodical_reward_rate_decrease_enabled</a>(): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">spec_periodical_reward_rate_decrease_enabled</a>();
</code></pre>




<a id="0x1_features_spec_partial_governance_voting_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_partial_governance_voting_enabled">spec_partial_governance_voting_enabled</a>(): bool {
   <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(<a href="features.md#0x1_features_PARTIAL_GOVERNANCE_VOTING">PARTIAL_GOVERNANCE_VOTING</a>)
}
</code></pre>



<a id="@Specification_1_partial_governance_voting_enabled"></a>

### Function `partial_governance_voting_enabled`


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_partial_governance_voting_enabled">partial_governance_voting_enabled</a>(): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="features.md#0x1_features_spec_partial_governance_voting_enabled">spec_partial_governance_voting_enabled</a>();
</code></pre>



<a id="@Specification_1_module_event_enabled"></a>

### Function `module_event_enabled`


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_module_event_enabled">module_event_enabled</a>(): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="features.md#0x1_features_spec_module_event_enabled">spec_module_event_enabled</a>();
</code></pre>




<a id="0x1_features_spec_abort_if_multisig_payload_mismatch_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_abort_if_multisig_payload_mismatch_enabled">spec_abort_if_multisig_payload_mismatch_enabled</a>(): bool {
   <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(<a href="features.md#0x1_features_ABORT_IF_MULTISIG_PAYLOAD_MISMATCH">ABORT_IF_MULTISIG_PAYLOAD_MISMATCH</a>)
}
</code></pre>



<a id="@Specification_1_abort_if_multisig_payload_mismatch_enabled"></a>

### Function `abort_if_multisig_payload_mismatch_enabled`


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_abort_if_multisig_payload_mismatch_enabled">abort_if_multisig_payload_mismatch_enabled</a>(): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="features.md#0x1_features_spec_abort_if_multisig_payload_mismatch_enabled">spec_abort_if_multisig_payload_mismatch_enabled</a>();
</code></pre>



<a id="@Specification_1_change_feature_flags_internal"></a>

### Function `change_feature_flags_internal`


<pre><code><b>fun</b> <a href="features.md#0x1_features_change_feature_flags_internal">change_feature_flags_internal</a>(framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>modifies</b> <b>global</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std);
<b>aborts_if</b> <a href="signer.md#0x1_signer_address_of">signer::address_of</a>(framework) != @std;
</code></pre>



<a id="@Specification_1_change_feature_flags_for_next_epoch"></a>

### Function `change_feature_flags_for_next_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_change_feature_flags_for_next_epoch">change_feature_flags_for_next_epoch</a>(framework: &<a href="signer.md#0x1_signer">signer</a>, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>aborts_if</b> <a href="signer.md#0x1_signer_address_of">signer::address_of</a>(framework) != @std;
<b>pragma</b> opaque;
<b>modifies</b> <b>global</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std);
<b>modifies</b> <b>global</b>&lt;<a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>&gt;(@std);
</code></pre>




<a id="0x1_features_spec_contains"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_contains">spec_contains</a>(<a href="features.md#0x1_features">features</a>: <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, feature: u64): bool {
   ((int2bv((((1 <b>as</b> u8) &lt;&lt; ((feature % (8 <b>as</b> u64)) <b>as</b> u64)) <b>as</b> u8)) <b>as</b> u8) & <a href="features.md#0x1_features">features</a>[feature/8] <b>as</b> u8) &gt; (0 <b>as</b> u8)
       && (feature / 8) &lt; len(<a href="features.md#0x1_features">features</a>)
}
</code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="features.md#0x1_features_on_new_epoch">on_new_epoch</a>(framework: &<a href="signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>requires</b> @std == <a href="signer.md#0x1_signer_address_of">signer::address_of</a>(framework);
<b>let</b> features_pending = <b>global</b>&lt;<a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>&gt;(@std).<a href="features.md#0x1_features">features</a>;
<b>let</b> <b>post</b> features_std = <b>global</b>&lt;<a href="features.md#0x1_features_Features">Features</a>&gt;(@std).<a href="features.md#0x1_features">features</a>;
<b>ensures</b> <b>exists</b>&lt;<a href="features.md#0x1_features_PendingFeatures">PendingFeatures</a>&gt;(@std) ==&gt; features_std == features_pending;
<b>aborts_if</b> <b>false</b>;
</code></pre>




<a id="0x1_features_spec_sha_512_and_ripemd_160_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_sha_512_and_ripemd_160_enabled">spec_sha_512_and_ripemd_160_enabled</a>(): bool {
   <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(<a href="features.md#0x1_features_SHA_512_AND_RIPEMD_160_NATIVES">SHA_512_AND_RIPEMD_160_NATIVES</a>)
}
</code></pre>



<a id="@Specification_1_is_enabled"></a>

### Function `is_enabled`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="features.md#0x1_features_is_enabled">is_enabled</a>(feature: u64): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(feature);
</code></pre>




<a id="0x1_features_spec_is_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(feature: u64): bool;
</code></pre>




<a id="0x1_features_spec_periodical_reward_rate_decrease_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">spec_periodical_reward_rate_decrease_enabled</a>(): bool {
   <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(<a href="features.md#0x1_features_PERIODICAL_REWARD_RATE_DECREASE">PERIODICAL_REWARD_RATE_DECREASE</a>)
}
</code></pre>




<a id="0x1_features_spec_fee_payer_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_fee_payer_enabled">spec_fee_payer_enabled</a>(): bool {
   <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(<a href="features.md#0x1_features_FEE_PAYER_ENABLED">FEE_PAYER_ENABLED</a>)
}
</code></pre>




<a id="0x1_features_spec_collect_and_distribute_gas_fees_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_collect_and_distribute_gas_fees_enabled">spec_collect_and_distribute_gas_fees_enabled</a>(): bool {
   <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(<a href="features.md#0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES">COLLECT_AND_DISTRIBUTE_GAS_FEES</a>)
}
</code></pre>




<a id="0x1_features_spec_module_event_enabled"></a>


<pre><code><b>fun</b> <a href="features.md#0x1_features_spec_module_event_enabled">spec_module_event_enabled</a>(): bool {
   <a href="features.md#0x1_features_spec_is_enabled">spec_is_enabled</a>(<a href="features.md#0x1_features_MODULE_EVENT">MODULE_EVENT</a>)
}
</code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code><b>fun</b> <a href="features.md#0x1_features_set">set</a>(<a href="features.md#0x1_features">features</a>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, feature: u64, <b>include</b>: bool)
</code></pre>




<pre><code><b>pragma</b> bv=b"0";
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> feature / 8 &lt; len(<a href="features.md#0x1_features">features</a>);
<b>ensures</b> <b>include</b> == <a href="features.md#0x1_features_spec_contains">spec_contains</a>(<a href="features.md#0x1_features">features</a>, feature);
</code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code><b>fun</b> <a href="features.md#0x1_features_contains">contains</a>(<a href="features.md#0x1_features">features</a>: &<a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, feature: u64): bool
</code></pre>




<pre><code><b>pragma</b> bv=b"0";
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="features.md#0x1_features_spec_contains">spec_contains</a>(<a href="features.md#0x1_features">features</a>, feature);
</code></pre>



<a id="@Specification_1_apply_diff"></a>

### Function `apply_diff`


<pre><code><b>fun</b> <a href="features.md#0x1_features_apply_diff">apply_diff</a>(<a href="features.md#0x1_features">features</a>: &<b>mut</b> <a href="vector.md#0x1_vector">vector</a>&lt;u8&gt;, enable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;, disable: <a href="vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>




<pre><code><b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] <b>forall</b> i in disable: !<a href="features.md#0x1_features_spec_contains">spec_contains</a>(<a href="features.md#0x1_features">features</a>, i);
<b>ensures</b> [abstract] <b>forall</b> i in enable: !<a href="vector.md#0x1_vector_spec_contains">vector::spec_contains</a>(disable, i)
    ==&gt; <a href="features.md#0x1_features_spec_contains">spec_contains</a>(<a href="features.md#0x1_features">features</a>, i);
<b>pragma</b> opaque;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
