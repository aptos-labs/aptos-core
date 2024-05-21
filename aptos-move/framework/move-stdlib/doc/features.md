
<a id="0x1_features"></a>

# Module `0x1::features`

Defines feature flags for Aptos. Those are used in Aptos specific implementations of features in
the Move stdlib, the Aptos stdlib, and the Aptos framework.

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
-  [Function `get_object_native_derived_address_feature`](#0x1_features_get_object_native_derived_address_feature)
-  [Function `object_native_derived_address_enabled`](#0x1_features_object_native_derived_address_enabled)
-  [Function `get_dispatchable_fungible_asset_feature`](#0x1_features_get_dispatchable_fungible_asset_feature)
-  [Function `dispatchable_fungible_asset_enabled`](#0x1_features_dispatchable_fungible_asset_enabled)
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
    -  [Function `change_feature_flags_internal`](#@Specification_1_change_feature_flags_internal)
    -  [Function `change_feature_flags_for_next_epoch`](#@Specification_1_change_feature_flags_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)
    -  [Function `is_enabled`](#@Specification_1_is_enabled)
    -  [Function `set`](#@Specification_1_set)
    -  [Function `contains`](#@Specification_1_contains)
    -  [Function `apply_diff`](#@Specification_1_apply_diff)


<pre><code>use 0x1::error;<br/>use 0x1::signer;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_features_Features"></a>

## Resource `Features`

The enabled features, represented by a bitset stored on chain.


<pre><code>struct Features has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>features: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_features_PendingFeatures"></a>

## Resource `PendingFeatures`

This resource holds the feature vec updates received in the current epoch.
On epoch change, the updates take effect and this buffer is cleared.


<pre><code>struct PendingFeatures has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>features: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_features_AGGREGATOR_V2_API"></a>

Whether the Aggregator V2 API feature is enabled.
Once enabled, the functions from aggregator_v2.move will be available for use.
Lifetime: transient


<pre><code>const AGGREGATOR_V2_API: u64 &#61; 30;<br/></code></pre>



<a id="0x1_features_AGGREGATOR_V2_DELAYED_FIELDS"></a>

Whether the Aggregator V2 delayed fields feature is enabled.
Once enabled, Aggregator V2 functions become parallel.
Lifetime: transient


<pre><code>const AGGREGATOR_V2_DELAYED_FIELDS: u64 &#61; 36;<br/></code></pre>



<a id="0x1_features_APTOS_STD_CHAIN_ID_NATIVES"></a>

Whether the new <code>aptos_stdlib::type_info::chain_id()</code> native for fetching the chain ID is enabled.
This is needed because of the introduction of a new native function.
Lifetime: transient


<pre><code>const APTOS_STD_CHAIN_ID_NATIVES: u64 &#61; 4;<br/></code></pre>



<a id="0x1_features_APTOS_UNIQUE_IDENTIFIERS"></a>

Whether enable MOVE functions to call create_auid method to create AUIDs.
Lifetime: transient


<pre><code>const APTOS_UNIQUE_IDENTIFIERS: u64 &#61; 23;<br/></code></pre>



<a id="0x1_features_BLAKE2B_256_NATIVE"></a>

Whether the new BLAKE2B-256 hash function native is enabled.
This is needed because of the introduction of new native function(s).
Lifetime: transient


<pre><code>const BLAKE2B_256_NATIVE: u64 &#61; 8;<br/></code></pre>



<a id="0x1_features_BLS12_381_STRUCTURES"></a>

Whether the generic algebra implementation for BLS12381 operations are enabled.

Lifetime: transient


<pre><code>const BLS12_381_STRUCTURES: u64 &#61; 13;<br/></code></pre>



<a id="0x1_features_BN254_STRUCTURES"></a>

Whether the generic algebra implementation for BN254 operations are enabled.

Lifetime: transient


<pre><code>const BN254_STRUCTURES: u64 &#61; 43;<br/></code></pre>



<a id="0x1_features_BULLETPROOFS_NATIVES"></a>

Whether the Bulletproofs zero-knowledge range proof module is enabled, and the related native function is
available. This is needed because of the introduction of a new native function.
Lifetime: transient


<pre><code>const BULLETPROOFS_NATIVES: u64 &#61; 24;<br/></code></pre>



<a id="0x1_features_CHARGE_INVARIANT_VIOLATION"></a>

Charge invariant violation error.
Lifetime: transient


<pre><code>const CHARGE_INVARIANT_VIOLATION: u64 &#61; 20;<br/></code></pre>



<a id="0x1_features_CODE_DEPENDENCY_CHECK"></a>

Whether validation of package dependencies is enabled, and the related native function is
available. This is needed because of introduction of a new native function.
Lifetime: transient


<pre><code>const CODE_DEPENDENCY_CHECK: u64 &#61; 1;<br/></code></pre>



<a id="0x1_features_COIN_TO_FUNGIBLE_ASSET_MIGRATION"></a>

Whether migration from coin to fungible asset feature is enabled.

Lifetime: transient


<pre><code>const COIN_TO_FUNGIBLE_ASSET_MIGRATION: u64 &#61; 60;<br/></code></pre>



<a id="0x1_features_COLLECT_AND_DISTRIBUTE_GAS_FEES"></a>

Whether gas fees are collected and distributed to the block proposers.
Lifetime: transient


<pre><code>const COLLECT_AND_DISTRIBUTE_GAS_FEES: u64 &#61; 6;<br/></code></pre>



<a id="0x1_features_COMMISSION_CHANGE_DELEGATION_POOL"></a>

Whether the operator commission rate change in delegation pool is enabled.
Lifetime: transient


<pre><code>const COMMISSION_CHANGE_DELEGATION_POOL: u64 &#61; 42;<br/></code></pre>



<a id="0x1_features_CONCURRENT_FUNGIBLE_ASSETS"></a>

Whether enable Fungible Asset creation
to create higher throughput concurrent variants.
Lifetime: transient


<pre><code>const CONCURRENT_FUNGIBLE_ASSETS: u64 &#61; 50;<br/></code></pre>



<a id="0x1_features_CONCURRENT_TOKEN_V2"></a>

Whether enable TokenV2 collection creation and Fungible Asset creation
to create higher throughput concurrent variants.
Lifetime: transient


<pre><code>const CONCURRENT_TOKEN_V2: u64 &#61; 37;<br/></code></pre>



<a id="0x1_features_CRYPTOGRAPHY_ALGEBRA_NATIVES"></a>

Whether generic algebra basic operation support in <code>crypto_algebra.move</code> are enabled.

Lifetime: transient


<pre><code>const CRYPTOGRAPHY_ALGEBRA_NATIVES: u64 &#61; 12;<br/></code></pre>



<a id="0x1_features_DELEGATION_POOLS"></a>

Whether delegation pools are enabled.
Lifetime: transient


<pre><code>const DELEGATION_POOLS: u64 &#61; 11;<br/></code></pre>



<a id="0x1_features_DELEGATION_POOL_ALLOWLISTING"></a>

Whether delegators allowlisting for delegation pools is supported.
Lifetime: transient


<pre><code>const DELEGATION_POOL_ALLOWLISTING: u64 &#61; 56;<br/></code></pre>



<a id="0x1_features_DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING"></a>

Whether enable paritial governance voting on delegation_pool.
Lifetime: transient


<pre><code>const DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING: u64 &#61; 21;<br/></code></pre>



<a id="0x1_features_DISPATCHABLE_FUNGIBLE_ASSET"></a>

Whether the dispatchable fungible asset standard feature is enabled.

Lifetime: transient


<pre><code>const DISPATCHABLE_FUNGIBLE_ASSET: u64 &#61; 63;<br/></code></pre>



<a id="0x1_features_EAPI_DISABLED"></a>



<pre><code>const EAPI_DISABLED: u64 &#61; 2;<br/></code></pre>



<a id="0x1_features_ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH"></a>

Whether native_public_key_validate aborts when a public key of the wrong length is given
Lifetime: ephemeral


<pre><code>const ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH: u64 &#61; 14;<br/></code></pre>



<a id="0x1_features_EFRAMEWORK_SIGNER_NEEDED"></a>

The provided signer has not a framework address.


<pre><code>const EFRAMEWORK_SIGNER_NEEDED: u64 &#61; 1;<br/></code></pre>



<a id="0x1_features_EINVALID_FEATURE"></a>



<pre><code>const EINVALID_FEATURE: u64 &#61; 1;<br/></code></pre>



<a id="0x1_features_FEE_PAYER_ACCOUNT_OPTIONAL"></a>



<pre><code>const FEE_PAYER_ACCOUNT_OPTIONAL: u64 &#61; 35;<br/></code></pre>



<a id="0x1_features_FEE_PAYER_ENABLED"></a>

Whether alternate gas payer is supported
Lifetime: transient


<pre><code>const FEE_PAYER_ENABLED: u64 &#61; 22;<br/></code></pre>



<a id="0x1_features_JWK_CONSENSUS"></a>

Deprecated by <code>aptos_framework::jwk_consensus_config::JWKConsensusConfig</code>.


<pre><code>const JWK_CONSENSUS: u64 &#61; 49;<br/></code></pre>



<a id="0x1_features_KEYLESS_ACCOUNTS"></a>

Whether the OIDB feature is enabled, possibly with the ZK-less verification mode.

Lifetime: transient


<pre><code>const KEYLESS_ACCOUNTS: u64 &#61; 46;<br/></code></pre>



<a id="0x1_features_KEYLESS_ACCOUNTS_WITH_PASSKEYS"></a>

Whether keyless accounts support passkey-based ephemeral signatures.

Lifetime: transient


<pre><code>const KEYLESS_ACCOUNTS_WITH_PASSKEYS: u64 &#61; 54;<br/></code></pre>



<a id="0x1_features_KEYLESS_BUT_ZKLESS_ACCOUNTS"></a>

Whether the ZK-less mode of the keyless accounts feature is enabled.

Lifetime: transient


<pre><code>const KEYLESS_BUT_ZKLESS_ACCOUNTS: u64 &#61; 47;<br/></code></pre>



<a id="0x1_features_LIMIT_MAX_IDENTIFIER_LENGTH"></a>



<pre><code>const LIMIT_MAX_IDENTIFIER_LENGTH: u64 &#61; 38;<br/></code></pre>



<a id="0x1_features_MAX_OBJECT_NESTING_CHECK"></a>

Whether checking the maximum object nesting is enabled.


<pre><code>const MAX_OBJECT_NESTING_CHECK: u64 &#61; 53;<br/></code></pre>



<a id="0x1_features_MODULE_EVENT"></a>

Whether emit function in <code>event.move</code> are enabled for module events.

Lifetime: transient


<pre><code>const MODULE_EVENT: u64 &#61; 26;<br/></code></pre>



<a id="0x1_features_MODULE_EVENT_MIGRATION"></a>

Whether aptos_framwork enables the behavior of module event migration.

Lifetime: transient


<pre><code>const MODULE_EVENT_MIGRATION: u64 &#61; 57;<br/></code></pre>



<a id="0x1_features_MULTISIG_ACCOUNTS"></a>

Whether multisig accounts (different from accounts with multi-ed25519 auth keys) are enabled.


<pre><code>const MULTISIG_ACCOUNTS: u64 &#61; 10;<br/></code></pre>



<a id="0x1_features_MULTISIG_V2_ENHANCEMENT"></a>

Whether the Multisig V2 enhancement feature is enabled.

Lifetime: transient


<pre><code>const MULTISIG_V2_ENHANCEMENT: u64 &#61; 55;<br/></code></pre>



<a id="0x1_features_MULTI_ED25519_PK_VALIDATE_V2_NATIVES"></a>

Whether the new <code>aptos_stdlib::multi_ed25519::public_key_validate_internal_v2()</code> native is enabled.
This is needed because of the introduction of a new native function.
Lifetime: transient


<pre><code>const MULTI_ED25519_PK_VALIDATE_V2_NATIVES: u64 &#61; 7;<br/></code></pre>



<a id="0x1_features_OBJECT_CODE_DEPLOYMENT"></a>

Whether deploying to objects is enabled.


<pre><code>const OBJECT_CODE_DEPLOYMENT: u64 &#61; 52;<br/></code></pre>



<a id="0x1_features_OBJECT_NATIVE_DERIVED_ADDRESS"></a>

Whether we use more efficient native implementation of computing object derived address


<pre><code>const OBJECT_NATIVE_DERIVED_ADDRESS: u64 &#61; 62;<br/></code></pre>



<a id="0x1_features_OPERATOR_BENEFICIARY_CHANGE"></a>

Whether allow changing beneficiaries for operators.
Lifetime: transient


<pre><code>const OPERATOR_BENEFICIARY_CHANGE: u64 &#61; 39;<br/></code></pre>



<a id="0x1_features_PARTIAL_GOVERNANCE_VOTING"></a>

Whether enable paritial governance voting on aptos_governance.
Lifetime: transient


<pre><code>const PARTIAL_GOVERNANCE_VOTING: u64 &#61; 17;<br/></code></pre>



<a id="0x1_features_PERIODICAL_REWARD_RATE_DECREASE"></a>

Whether reward rate decreases periodically.
Lifetime: transient


<pre><code>const PERIODICAL_REWARD_RATE_DECREASE: u64 &#61; 16;<br/></code></pre>



<a id="0x1_features_PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS"></a>



<pre><code>const PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS: u64 &#61; 61;<br/></code></pre>



<a id="0x1_features_RECONFIGURE_WITH_DKG"></a>

Deprecated by <code>aptos_framework::randomness_config::RandomnessConfig</code>.


<pre><code>const RECONFIGURE_WITH_DKG: u64 &#61; 45;<br/></code></pre>



<a id="0x1_features_RESOURCE_GROUPS"></a>

Whether resource groups are enabled.
This is needed because of new attributes for structs and a change in storage representation.


<pre><code>const RESOURCE_GROUPS: u64 &#61; 9;<br/></code></pre>



<a id="0x1_features_RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET"></a>



<pre><code>const RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET: u64 &#61; 41;<br/></code></pre>



<a id="0x1_features_SAFER_METADATA"></a>



<pre><code>const SAFER_METADATA: u64 &#61; 32;<br/></code></pre>



<a id="0x1_features_SAFER_RESOURCE_GROUPS"></a>



<pre><code>const SAFER_RESOURCE_GROUPS: u64 &#61; 31;<br/></code></pre>



<a id="0x1_features_SHA_512_AND_RIPEMD_160_NATIVES"></a>

Whether the new SHA2-512, SHA3-512 and RIPEMD-160 hash function natives are enabled.
This is needed because of the introduction of new native functions.
Lifetime: transient


<pre><code>const SHA_512_AND_RIPEMD_160_NATIVES: u64 &#61; 3;<br/></code></pre>



<a id="0x1_features_SIGNATURE_CHECKER_V2_SCRIPT_FIX"></a>

Whether the fix for a counting bug in the script path of the signature checker pass is enabled.
Lifetime: transient


<pre><code>const SIGNATURE_CHECKER_V2_SCRIPT_FIX: u64 &#61; 29;<br/></code></pre>



<a id="0x1_features_SIGNER_NATIVE_FORMAT_FIX"></a>

Fix the native formatter for signer.
Lifetime: transient


<pre><code>const SIGNER_NATIVE_FORMAT_FIX: u64 &#61; 25;<br/></code></pre>



<a id="0x1_features_SINGLE_SENDER_AUTHENTICATOR"></a>



<pre><code>const SINGLE_SENDER_AUTHENTICATOR: u64 &#61; 33;<br/></code></pre>



<a id="0x1_features_SPONSORED_AUTOMATIC_ACCOUNT_CREATION"></a>

Whether the automatic creation of accounts is enabled for sponsored transactions.
Lifetime: transient


<pre><code>const SPONSORED_AUTOMATIC_ACCOUNT_CREATION: u64 &#61; 34;<br/></code></pre>



<a id="0x1_features_STRUCT_CONSTRUCTORS"></a>

Whether struct constructors are enabled

Lifetime: transient


<pre><code>const STRUCT_CONSTRUCTORS: u64 &#61; 15;<br/></code></pre>



<a id="0x1_features_TRANSACTION_CONTEXT_EXTENSION"></a>

Whether the transaction context extension is enabled. This feature allows the module
<code>transaction_context</code> to provide contextual information about the user transaction.

Lifetime: transient


<pre><code>const TRANSACTION_CONTEXT_EXTENSION: u64 &#61; 59;<br/></code></pre>



<a id="0x1_features_TREAT_FRIEND_AS_PRIVATE"></a>

Whether during upgrade compatibility checking, friend functions should be treated similar like
private functions.
Lifetime: permanent


<pre><code>const TREAT_FRIEND_AS_PRIVATE: u64 &#61; 2;<br/></code></pre>



<a id="0x1_features_VM_BINARY_FORMAT_V6"></a>

Whether to allow the use of binary format version v6.
Lifetime: transient


<pre><code>const VM_BINARY_FORMAT_V6: u64 &#61; 5;<br/></code></pre>



<a id="0x1_features_VM_BINARY_FORMAT_V7"></a>



<pre><code>const VM_BINARY_FORMAT_V7: u64 &#61; 40;<br/></code></pre>



<a id="0x1_features_code_dependency_check_enabled"></a>

## Function `code_dependency_check_enabled`



<pre><code>public fun code_dependency_check_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun code_dependency_check_enabled(): bool acquires Features &#123;<br/>    is_enabled(CODE_DEPENDENCY_CHECK)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_treat_friend_as_private"></a>

## Function `treat_friend_as_private`



<pre><code>public fun treat_friend_as_private(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun treat_friend_as_private(): bool acquires Features &#123;<br/>    is_enabled(TREAT_FRIEND_AS_PRIVATE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_sha_512_and_ripemd_160_feature"></a>

## Function `get_sha_512_and_ripemd_160_feature`



<pre><code>public fun get_sha_512_and_ripemd_160_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_sha_512_and_ripemd_160_feature(): u64 &#123; SHA_512_AND_RIPEMD_160_NATIVES &#125;<br/></code></pre>



</details>

<a id="0x1_features_sha_512_and_ripemd_160_enabled"></a>

## Function `sha_512_and_ripemd_160_enabled`



<pre><code>public fun sha_512_and_ripemd_160_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sha_512_and_ripemd_160_enabled(): bool acquires Features &#123;<br/>    is_enabled(SHA_512_AND_RIPEMD_160_NATIVES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_aptos_stdlib_chain_id_feature"></a>

## Function `get_aptos_stdlib_chain_id_feature`



<pre><code>public fun get_aptos_stdlib_chain_id_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_aptos_stdlib_chain_id_feature(): u64 &#123; APTOS_STD_CHAIN_ID_NATIVES &#125;<br/></code></pre>



</details>

<a id="0x1_features_aptos_stdlib_chain_id_enabled"></a>

## Function `aptos_stdlib_chain_id_enabled`



<pre><code>public fun aptos_stdlib_chain_id_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun aptos_stdlib_chain_id_enabled(): bool acquires Features &#123;<br/>    is_enabled(APTOS_STD_CHAIN_ID_NATIVES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_vm_binary_format_v6"></a>

## Function `get_vm_binary_format_v6`



<pre><code>public fun get_vm_binary_format_v6(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_vm_binary_format_v6(): u64 &#123; VM_BINARY_FORMAT_V6 &#125;<br/></code></pre>



</details>

<a id="0x1_features_allow_vm_binary_format_v6"></a>

## Function `allow_vm_binary_format_v6`



<pre><code>public fun allow_vm_binary_format_v6(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun allow_vm_binary_format_v6(): bool acquires Features &#123;<br/>    is_enabled(VM_BINARY_FORMAT_V6)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_collect_and_distribute_gas_fees_feature"></a>

## Function `get_collect_and_distribute_gas_fees_feature`



<pre><code>public fun get_collect_and_distribute_gas_fees_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_collect_and_distribute_gas_fees_feature(): u64 &#123; COLLECT_AND_DISTRIBUTE_GAS_FEES &#125;<br/></code></pre>



</details>

<a id="0x1_features_collect_and_distribute_gas_fees"></a>

## Function `collect_and_distribute_gas_fees`



<pre><code>public fun collect_and_distribute_gas_fees(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun collect_and_distribute_gas_fees(): bool acquires Features &#123;<br/>    is_enabled(COLLECT_AND_DISTRIBUTE_GAS_FEES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_multi_ed25519_pk_validate_v2_feature"></a>

## Function `multi_ed25519_pk_validate_v2_feature`



<pre><code>public fun multi_ed25519_pk_validate_v2_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multi_ed25519_pk_validate_v2_feature(): u64 &#123; MULTI_ED25519_PK_VALIDATE_V2_NATIVES &#125;<br/></code></pre>



</details>

<a id="0x1_features_multi_ed25519_pk_validate_v2_enabled"></a>

## Function `multi_ed25519_pk_validate_v2_enabled`



<pre><code>public fun multi_ed25519_pk_validate_v2_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multi_ed25519_pk_validate_v2_enabled(): bool acquires Features &#123;<br/>    is_enabled(MULTI_ED25519_PK_VALIDATE_V2_NATIVES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_blake2b_256_feature"></a>

## Function `get_blake2b_256_feature`



<pre><code>public fun get_blake2b_256_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_blake2b_256_feature(): u64 &#123; BLAKE2B_256_NATIVE &#125;<br/></code></pre>



</details>

<a id="0x1_features_blake2b_256_enabled"></a>

## Function `blake2b_256_enabled`



<pre><code>public fun blake2b_256_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun blake2b_256_enabled(): bool acquires Features &#123;<br/>    is_enabled(BLAKE2B_256_NATIVE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_resource_groups_feature"></a>

## Function `get_resource_groups_feature`



<pre><code>public fun get_resource_groups_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_resource_groups_feature(): u64 &#123; RESOURCE_GROUPS &#125;<br/></code></pre>



</details>

<a id="0x1_features_resource_groups_enabled"></a>

## Function `resource_groups_enabled`



<pre><code>public fun resource_groups_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun resource_groups_enabled(): bool acquires Features &#123;<br/>    is_enabled(RESOURCE_GROUPS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_multisig_accounts_feature"></a>

## Function `get_multisig_accounts_feature`



<pre><code>public fun get_multisig_accounts_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_multisig_accounts_feature(): u64 &#123; MULTISIG_ACCOUNTS &#125;<br/></code></pre>



</details>

<a id="0x1_features_multisig_accounts_enabled"></a>

## Function `multisig_accounts_enabled`



<pre><code>public fun multisig_accounts_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multisig_accounts_enabled(): bool acquires Features &#123;<br/>    is_enabled(MULTISIG_ACCOUNTS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_delegation_pools_feature"></a>

## Function `get_delegation_pools_feature`



<pre><code>public fun get_delegation_pools_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegation_pools_feature(): u64 &#123; DELEGATION_POOLS &#125;<br/></code></pre>



</details>

<a id="0x1_features_delegation_pools_enabled"></a>

## Function `delegation_pools_enabled`



<pre><code>public fun delegation_pools_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegation_pools_enabled(): bool acquires Features &#123;<br/>    is_enabled(DELEGATION_POOLS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_cryptography_algebra_natives_feature"></a>

## Function `get_cryptography_algebra_natives_feature`



<pre><code>public fun get_cryptography_algebra_natives_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_cryptography_algebra_natives_feature(): u64 &#123; CRYPTOGRAPHY_ALGEBRA_NATIVES &#125;<br/></code></pre>



</details>

<a id="0x1_features_cryptography_algebra_enabled"></a>

## Function `cryptography_algebra_enabled`



<pre><code>public fun cryptography_algebra_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun cryptography_algebra_enabled(): bool acquires Features &#123;<br/>    is_enabled(CRYPTOGRAPHY_ALGEBRA_NATIVES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_bls12_381_strutures_feature"></a>

## Function `get_bls12_381_strutures_feature`



<pre><code>public fun get_bls12_381_strutures_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_bls12_381_strutures_feature(): u64 &#123; BLS12_381_STRUCTURES &#125;<br/></code></pre>



</details>

<a id="0x1_features_bls12_381_structures_enabled"></a>

## Function `bls12_381_structures_enabled`



<pre><code>public fun bls12_381_structures_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun bls12_381_structures_enabled(): bool acquires Features &#123;<br/>    is_enabled(BLS12_381_STRUCTURES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_periodical_reward_rate_decrease_feature"></a>

## Function `get_periodical_reward_rate_decrease_feature`



<pre><code>public fun get_periodical_reward_rate_decrease_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_periodical_reward_rate_decrease_feature(): u64 &#123; PERIODICAL_REWARD_RATE_DECREASE &#125;<br/></code></pre>



</details>

<a id="0x1_features_periodical_reward_rate_decrease_enabled"></a>

## Function `periodical_reward_rate_decrease_enabled`



<pre><code>public fun periodical_reward_rate_decrease_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun periodical_reward_rate_decrease_enabled(): bool acquires Features &#123;<br/>    is_enabled(PERIODICAL_REWARD_RATE_DECREASE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_partial_governance_voting"></a>

## Function `get_partial_governance_voting`



<pre><code>public fun get_partial_governance_voting(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_partial_governance_voting(): u64 &#123; PARTIAL_GOVERNANCE_VOTING &#125;<br/></code></pre>



</details>

<a id="0x1_features_partial_governance_voting_enabled"></a>

## Function `partial_governance_voting_enabled`



<pre><code>public fun partial_governance_voting_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun partial_governance_voting_enabled(): bool acquires Features &#123;<br/>    is_enabled(PARTIAL_GOVERNANCE_VOTING)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_delegation_pool_partial_governance_voting"></a>

## Function `get_delegation_pool_partial_governance_voting`



<pre><code>public fun get_delegation_pool_partial_governance_voting(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegation_pool_partial_governance_voting(): u64 &#123; DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING &#125;<br/></code></pre>



</details>

<a id="0x1_features_delegation_pool_partial_governance_voting_enabled"></a>

## Function `delegation_pool_partial_governance_voting_enabled`



<pre><code>public fun delegation_pool_partial_governance_voting_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegation_pool_partial_governance_voting_enabled(): bool acquires Features &#123;<br/>    is_enabled(DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_fee_payer_enabled"></a>

## Function `fee_payer_enabled`



<pre><code>public fun fee_payer_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun fee_payer_enabled(): bool acquires Features &#123;<br/>    is_enabled(FEE_PAYER_ENABLED)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_auids"></a>

## Function `get_auids`



<pre><code>public fun get_auids(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_auids(): u64 &#123; APTOS_UNIQUE_IDENTIFIERS &#125;<br/></code></pre>



</details>

<a id="0x1_features_auids_enabled"></a>

## Function `auids_enabled`



<pre><code>public fun auids_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun auids_enabled(): bool acquires Features &#123;<br/>    is_enabled(APTOS_UNIQUE_IDENTIFIERS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_bulletproofs_feature"></a>

## Function `get_bulletproofs_feature`



<pre><code>public fun get_bulletproofs_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_bulletproofs_feature(): u64 &#123; BULLETPROOFS_NATIVES &#125;<br/></code></pre>



</details>

<a id="0x1_features_bulletproofs_enabled"></a>

## Function `bulletproofs_enabled`



<pre><code>public fun bulletproofs_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun bulletproofs_enabled(): bool acquires Features &#123;<br/>    is_enabled(BULLETPROOFS_NATIVES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_signer_native_format_fix_feature"></a>

## Function `get_signer_native_format_fix_feature`



<pre><code>public fun get_signer_native_format_fix_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_signer_native_format_fix_feature(): u64 &#123; SIGNER_NATIVE_FORMAT_FIX &#125;<br/></code></pre>



</details>

<a id="0x1_features_signer_native_format_fix_enabled"></a>

## Function `signer_native_format_fix_enabled`



<pre><code>public fun signer_native_format_fix_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun signer_native_format_fix_enabled(): bool acquires Features &#123;<br/>    is_enabled(SIGNER_NATIVE_FORMAT_FIX)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_module_event_feature"></a>

## Function `get_module_event_feature`



<pre><code>public fun get_module_event_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_module_event_feature(): u64 &#123; MODULE_EVENT &#125;<br/></code></pre>



</details>

<a id="0x1_features_module_event_enabled"></a>

## Function `module_event_enabled`



<pre><code>public fun module_event_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun module_event_enabled(): bool acquires Features &#123;<br/>    is_enabled(MODULE_EVENT)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_aggregator_v2_api_feature"></a>

## Function `get_aggregator_v2_api_feature`



<pre><code>public fun get_aggregator_v2_api_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_aggregator_v2_api_feature(): u64 &#123; AGGREGATOR_V2_API &#125;<br/></code></pre>



</details>

<a id="0x1_features_aggregator_v2_api_enabled"></a>

## Function `aggregator_v2_api_enabled`



<pre><code>public fun aggregator_v2_api_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun aggregator_v2_api_enabled(): bool acquires Features &#123;<br/>    is_enabled(AGGREGATOR_V2_API)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_aggregator_snapshots_feature"></a>

## Function `get_aggregator_snapshots_feature`



<pre><code>&#35;[deprecated]<br/>public fun get_aggregator_snapshots_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_aggregator_snapshots_feature(): u64 &#123;<br/>    abort error::invalid_argument(EINVALID_FEATURE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_aggregator_snapshots_enabled"></a>

## Function `aggregator_snapshots_enabled`



<pre><code>&#35;[deprecated]<br/>public fun aggregator_snapshots_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun aggregator_snapshots_enabled(): bool &#123;<br/>    abort error::invalid_argument(EINVALID_FEATURE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_sponsored_automatic_account_creation"></a>

## Function `get_sponsored_automatic_account_creation`



<pre><code>public fun get_sponsored_automatic_account_creation(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_sponsored_automatic_account_creation(): u64 &#123; SPONSORED_AUTOMATIC_ACCOUNT_CREATION &#125;<br/></code></pre>



</details>

<a id="0x1_features_sponsored_automatic_account_creation_enabled"></a>

## Function `sponsored_automatic_account_creation_enabled`



<pre><code>public fun sponsored_automatic_account_creation_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun sponsored_automatic_account_creation_enabled(): bool acquires Features &#123;<br/>    is_enabled(SPONSORED_AUTOMATIC_ACCOUNT_CREATION)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_concurrent_token_v2_feature"></a>

## Function `get_concurrent_token_v2_feature`



<pre><code>public fun get_concurrent_token_v2_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_concurrent_token_v2_feature(): u64 &#123; CONCURRENT_TOKEN_V2 &#125;<br/></code></pre>



</details>

<a id="0x1_features_concurrent_token_v2_enabled"></a>

## Function `concurrent_token_v2_enabled`



<pre><code>public fun concurrent_token_v2_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun concurrent_token_v2_enabled(): bool acquires Features &#123;<br/>    // concurrent token v2 cannot be used if aggregator v2 api is not enabled.<br/>    is_enabled(CONCURRENT_TOKEN_V2) &amp;&amp; aggregator_v2_api_enabled()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_concurrent_assets_feature"></a>

## Function `get_concurrent_assets_feature`



<pre><code>&#35;[deprecated]<br/>public fun get_concurrent_assets_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_concurrent_assets_feature(): u64 &#123;<br/>    abort error::invalid_argument(EINVALID_FEATURE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_concurrent_assets_enabled"></a>

## Function `concurrent_assets_enabled`



<pre><code>&#35;[deprecated]<br/>public fun concurrent_assets_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun concurrent_assets_enabled(): bool &#123;<br/>    abort error::invalid_argument(EINVALID_FEATURE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_operator_beneficiary_change_feature"></a>

## Function `get_operator_beneficiary_change_feature`



<pre><code>public fun get_operator_beneficiary_change_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_operator_beneficiary_change_feature(): u64 &#123; OPERATOR_BENEFICIARY_CHANGE &#125;<br/></code></pre>



</details>

<a id="0x1_features_operator_beneficiary_change_enabled"></a>

## Function `operator_beneficiary_change_enabled`



<pre><code>public fun operator_beneficiary_change_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun operator_beneficiary_change_enabled(): bool acquires Features &#123;<br/>    is_enabled(OPERATOR_BENEFICIARY_CHANGE)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_commission_change_delegation_pool_feature"></a>

## Function `get_commission_change_delegation_pool_feature`



<pre><code>public fun get_commission_change_delegation_pool_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_commission_change_delegation_pool_feature(): u64 &#123; COMMISSION_CHANGE_DELEGATION_POOL &#125;<br/></code></pre>



</details>

<a id="0x1_features_commission_change_delegation_pool_enabled"></a>

## Function `commission_change_delegation_pool_enabled`



<pre><code>public fun commission_change_delegation_pool_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun commission_change_delegation_pool_enabled(): bool acquires Features &#123;<br/>    is_enabled(COMMISSION_CHANGE_DELEGATION_POOL)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_bn254_strutures_feature"></a>

## Function `get_bn254_strutures_feature`



<pre><code>public fun get_bn254_strutures_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_bn254_strutures_feature(): u64 &#123; BN254_STRUCTURES &#125;<br/></code></pre>



</details>

<a id="0x1_features_bn254_structures_enabled"></a>

## Function `bn254_structures_enabled`



<pre><code>public fun bn254_structures_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun bn254_structures_enabled(): bool acquires Features &#123;<br/>    is_enabled(BN254_STRUCTURES)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_reconfigure_with_dkg_feature"></a>

## Function `get_reconfigure_with_dkg_feature`



<pre><code>public fun get_reconfigure_with_dkg_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_reconfigure_with_dkg_feature(): u64 &#123; RECONFIGURE_WITH_DKG &#125;<br/></code></pre>



</details>

<a id="0x1_features_reconfigure_with_dkg_enabled"></a>

## Function `reconfigure_with_dkg_enabled`



<pre><code>public fun reconfigure_with_dkg_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun reconfigure_with_dkg_enabled(): bool acquires Features &#123;<br/>    is_enabled(RECONFIGURE_WITH_DKG)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_keyless_accounts_feature"></a>

## Function `get_keyless_accounts_feature`



<pre><code>public fun get_keyless_accounts_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_keyless_accounts_feature(): u64 &#123; KEYLESS_ACCOUNTS &#125;<br/></code></pre>



</details>

<a id="0x1_features_keyless_accounts_enabled"></a>

## Function `keyless_accounts_enabled`



<pre><code>public fun keyless_accounts_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keyless_accounts_enabled(): bool acquires Features &#123;<br/>    is_enabled(KEYLESS_ACCOUNTS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_keyless_but_zkless_accounts_feature"></a>

## Function `get_keyless_but_zkless_accounts_feature`



<pre><code>public fun get_keyless_but_zkless_accounts_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_keyless_but_zkless_accounts_feature(): u64 &#123; KEYLESS_BUT_ZKLESS_ACCOUNTS &#125;<br/></code></pre>



</details>

<a id="0x1_features_keyless_but_zkless_accounts_feature_enabled"></a>

## Function `keyless_but_zkless_accounts_feature_enabled`



<pre><code>public fun keyless_but_zkless_accounts_feature_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keyless_but_zkless_accounts_feature_enabled(): bool acquires Features &#123;<br/>    is_enabled(KEYLESS_BUT_ZKLESS_ACCOUNTS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_jwk_consensus_feature"></a>

## Function `get_jwk_consensus_feature`



<pre><code>public fun get_jwk_consensus_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_jwk_consensus_feature(): u64 &#123; JWK_CONSENSUS &#125;<br/></code></pre>



</details>

<a id="0x1_features_jwk_consensus_enabled"></a>

## Function `jwk_consensus_enabled`



<pre><code>public fun jwk_consensus_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun jwk_consensus_enabled(): bool acquires Features &#123;<br/>    is_enabled(JWK_CONSENSUS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_concurrent_fungible_assets_feature"></a>

## Function `get_concurrent_fungible_assets_feature`



<pre><code>public fun get_concurrent_fungible_assets_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_concurrent_fungible_assets_feature(): u64 &#123; CONCURRENT_FUNGIBLE_ASSETS &#125;<br/></code></pre>



</details>

<a id="0x1_features_concurrent_fungible_assets_enabled"></a>

## Function `concurrent_fungible_assets_enabled`



<pre><code>public fun concurrent_fungible_assets_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun concurrent_fungible_assets_enabled(): bool acquires Features &#123;<br/>    // concurrent fungible assets cannot be used if aggregator v2 api is not enabled.<br/>    is_enabled(CONCURRENT_FUNGIBLE_ASSETS) &amp;&amp; aggregator_v2_api_enabled()<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_is_object_code_deployment_enabled"></a>

## Function `is_object_code_deployment_enabled`



<pre><code>public fun is_object_code_deployment_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_object_code_deployment_enabled(): bool acquires Features &#123;<br/>    is_enabled(OBJECT_CODE_DEPLOYMENT)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_max_object_nesting_check_feature"></a>

## Function `get_max_object_nesting_check_feature`



<pre><code>public fun get_max_object_nesting_check_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_max_object_nesting_check_feature(): u64 &#123; MAX_OBJECT_NESTING_CHECK &#125;<br/></code></pre>



</details>

<a id="0x1_features_max_object_nesting_check_enabled"></a>

## Function `max_object_nesting_check_enabled`



<pre><code>public fun max_object_nesting_check_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun max_object_nesting_check_enabled(): bool acquires Features &#123;<br/>    is_enabled(MAX_OBJECT_NESTING_CHECK)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_keyless_accounts_with_passkeys_feature"></a>

## Function `get_keyless_accounts_with_passkeys_feature`



<pre><code>public fun get_keyless_accounts_with_passkeys_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_keyless_accounts_with_passkeys_feature(): u64 &#123; KEYLESS_ACCOUNTS_WITH_PASSKEYS &#125;<br/></code></pre>



</details>

<a id="0x1_features_keyless_accounts_with_passkeys_feature_enabled"></a>

## Function `keyless_accounts_with_passkeys_feature_enabled`



<pre><code>public fun keyless_accounts_with_passkeys_feature_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun keyless_accounts_with_passkeys_feature_enabled(): bool acquires Features &#123;<br/>    is_enabled(KEYLESS_ACCOUNTS_WITH_PASSKEYS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_multisig_v2_enhancement_feature"></a>

## Function `get_multisig_v2_enhancement_feature`



<pre><code>public fun get_multisig_v2_enhancement_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_multisig_v2_enhancement_feature(): u64 &#123; MULTISIG_V2_ENHANCEMENT &#125;<br/></code></pre>



</details>

<a id="0x1_features_multisig_v2_enhancement_feature_enabled"></a>

## Function `multisig_v2_enhancement_feature_enabled`



<pre><code>public fun multisig_v2_enhancement_feature_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun multisig_v2_enhancement_feature_enabled(): bool acquires Features &#123;<br/>    is_enabled(MULTISIG_V2_ENHANCEMENT)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_delegation_pool_allowlisting_feature"></a>

## Function `get_delegation_pool_allowlisting_feature`



<pre><code>public fun get_delegation_pool_allowlisting_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_delegation_pool_allowlisting_feature(): u64 &#123; DELEGATION_POOL_ALLOWLISTING &#125;<br/></code></pre>



</details>

<a id="0x1_features_delegation_pool_allowlisting_enabled"></a>

## Function `delegation_pool_allowlisting_enabled`



<pre><code>public fun delegation_pool_allowlisting_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun delegation_pool_allowlisting_enabled(): bool acquires Features &#123;<br/>    is_enabled(DELEGATION_POOL_ALLOWLISTING)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_module_event_migration_feature"></a>

## Function `get_module_event_migration_feature`



<pre><code>public fun get_module_event_migration_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_module_event_migration_feature(): u64 &#123; MODULE_EVENT_MIGRATION &#125;<br/></code></pre>



</details>

<a id="0x1_features_module_event_migration_enabled"></a>

## Function `module_event_migration_enabled`



<pre><code>public fun module_event_migration_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun module_event_migration_enabled(): bool acquires Features &#123;<br/>    is_enabled(MODULE_EVENT_MIGRATION)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_transaction_context_extension_feature"></a>

## Function `get_transaction_context_extension_feature`



<pre><code>public fun get_transaction_context_extension_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_transaction_context_extension_feature(): u64 &#123; TRANSACTION_CONTEXT_EXTENSION &#125;<br/></code></pre>



</details>

<a id="0x1_features_transaction_context_extension_enabled"></a>

## Function `transaction_context_extension_enabled`



<pre><code>public fun transaction_context_extension_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun transaction_context_extension_enabled(): bool acquires Features &#123;<br/>    is_enabled(TRANSACTION_CONTEXT_EXTENSION)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_coin_to_fungible_asset_migration_feature"></a>

## Function `get_coin_to_fungible_asset_migration_feature`



<pre><code>public fun get_coin_to_fungible_asset_migration_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_coin_to_fungible_asset_migration_feature(): u64 &#123; COIN_TO_FUNGIBLE_ASSET_MIGRATION &#125;<br/></code></pre>



</details>

<a id="0x1_features_coin_to_fungible_asset_migration_feature_enabled"></a>

## Function `coin_to_fungible_asset_migration_feature_enabled`



<pre><code>public fun coin_to_fungible_asset_migration_feature_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun coin_to_fungible_asset_migration_feature_enabled(): bool acquires Features &#123;<br/>    is_enabled(COIN_TO_FUNGIBLE_ASSET_MIGRATION)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_primary_apt_fungible_store_at_user_address_feature"></a>

## Function `get_primary_apt_fungible_store_at_user_address_feature`



<pre><code>public fun get_primary_apt_fungible_store_at_user_address_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_primary_apt_fungible_store_at_user_address_feature(<br/>): u64 &#123; PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS &#125;<br/></code></pre>



</details>

<a id="0x1_features_primary_apt_fungible_store_at_user_address_enabled"></a>

## Function `primary_apt_fungible_store_at_user_address_enabled`



<pre><code>public fun primary_apt_fungible_store_at_user_address_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun primary_apt_fungible_store_at_user_address_enabled(): bool acquires Features &#123;<br/>    is_enabled(PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_object_native_derived_address_feature"></a>

## Function `get_object_native_derived_address_feature`



<pre><code>public fun get_object_native_derived_address_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_object_native_derived_address_feature(): u64 &#123; OBJECT_NATIVE_DERIVED_ADDRESS &#125;<br/></code></pre>



</details>

<a id="0x1_features_object_native_derived_address_enabled"></a>

## Function `object_native_derived_address_enabled`



<pre><code>public fun object_native_derived_address_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun object_native_derived_address_enabled(): bool acquires Features &#123;<br/>    is_enabled(OBJECT_NATIVE_DERIVED_ADDRESS)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_get_dispatchable_fungible_asset_feature"></a>

## Function `get_dispatchable_fungible_asset_feature`



<pre><code>public fun get_dispatchable_fungible_asset_feature(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_dispatchable_fungible_asset_feature(): u64 &#123; DISPATCHABLE_FUNGIBLE_ASSET &#125;<br/></code></pre>



</details>

<a id="0x1_features_dispatchable_fungible_asset_enabled"></a>

## Function `dispatchable_fungible_asset_enabled`



<pre><code>public fun dispatchable_fungible_asset_enabled(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun dispatchable_fungible_asset_enabled(): bool acquires Features &#123;<br/>    is_enabled(DISPATCHABLE_FUNGIBLE_ASSET)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_change_feature_flags"></a>

## Function `change_feature_flags`

Deprecated to prevent validator set changes during DKG.

Genesis/tests should use <code>change_feature_flags_internal()</code> for feature vec initialization.

Governance proposals should use <code>change_feature_flags_for_next_epoch()</code> to enable/disable features.


<pre><code>public fun change_feature_flags(_framework: &amp;signer, _enable: vector&lt;u64&gt;, _disable: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun change_feature_flags(_framework: &amp;signer, _enable: vector&lt;u64&gt;, _disable: vector&lt;u64&gt;) &#123;<br/>    abort (error::invalid_state(EAPI_DISABLED))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_change_feature_flags_internal"></a>

## Function `change_feature_flags_internal`

Update feature flags directly. Only used in genesis/tests.


<pre><code>fun change_feature_flags_internal(framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun change_feature_flags_internal(framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;) acquires Features &#123;<br/>    assert!(signer::address_of(framework) &#61;&#61; @std, error::permission_denied(EFRAMEWORK_SIGNER_NEEDED));<br/>    if (!exists&lt;Features&gt;(@std)) &#123;<br/>        move_to&lt;Features&gt;(framework, Features &#123; features: vector[] &#125;)<br/>    &#125;;<br/>    let features &#61; &amp;mut borrow_global_mut&lt;Features&gt;(@std).features;<br/>    vector::for_each_ref(&amp;enable, &#124;feature&#124; &#123;<br/>        set(features, &#42;feature, true);<br/>    &#125;);<br/>    vector::for_each_ref(&amp;disable, &#124;feature&#124; &#123;<br/>        set(features, &#42;feature, false);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_change_feature_flags_for_next_epoch"></a>

## Function `change_feature_flags_for_next_epoch`

Enable and disable features for the next epoch.


<pre><code>public fun change_feature_flags_for_next_epoch(framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun change_feature_flags_for_next_epoch(<br/>    framework: &amp;signer,<br/>    enable: vector&lt;u64&gt;,<br/>    disable: vector&lt;u64&gt;<br/>) acquires PendingFeatures, Features &#123;<br/>    assert!(signer::address_of(framework) &#61;&#61; @std, error::permission_denied(EFRAMEWORK_SIGNER_NEEDED));<br/><br/>    // Figure out the baseline feature vec that the diff will be applied to.<br/>    let new_feature_vec &#61; if (exists&lt;PendingFeatures&gt;(@std)) &#123;<br/>        // If there is a buffered feature vec, use it as the baseline.<br/>        let PendingFeatures &#123; features &#125; &#61; move_from&lt;PendingFeatures&gt;(@std);<br/>        features<br/>    &#125; else if (exists&lt;Features&gt;(@std)) &#123;<br/>        // Otherwise, use the currently effective feature flag vec as the baseline, if it exists.<br/>        borrow_global&lt;Features&gt;(@std).features<br/>    &#125; else &#123;<br/>        // Otherwise, use an empty feature vec.<br/>        vector[]<br/>    &#125;;<br/><br/>    // Apply the diff and save it to the buffer.<br/>    apply_diff(&amp;mut new_feature_vec, enable, disable);<br/>    move_to(framework, PendingFeatures &#123; features: new_feature_vec &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_on_new_epoch"></a>

## Function `on_new_epoch`

Apply all the pending feature flag changes. Should only be used at the end of a reconfiguration with DKG.

While the scope is public, it can only be usd in system transactions like <code>block_prologue</code> and governance proposals,
who have permission to set the flag that's checked in <code>extract()</code>.


<pre><code>public fun on_new_epoch(framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun on_new_epoch(framework: &amp;signer) acquires Features, PendingFeatures &#123;<br/>    ensure_framework_signer(framework);<br/>    if (exists&lt;PendingFeatures&gt;(@std)) &#123;<br/>        let PendingFeatures &#123; features &#125; &#61; move_from&lt;PendingFeatures&gt;(@std);<br/>        if (exists&lt;Features&gt;(@std)) &#123;<br/>            borrow_global_mut&lt;Features&gt;(@std).features &#61; features;<br/>        &#125; else &#123;<br/>            move_to(framework, Features &#123; features &#125;)<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_is_enabled"></a>

## Function `is_enabled`

Check whether the feature is enabled.


<pre><code>&#35;[view]<br/>public fun is_enabled(feature: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_enabled(feature: u64): bool acquires Features &#123;<br/>    exists&lt;Features&gt;(@std) &amp;&amp;<br/>        contains(&amp;borrow_global&lt;Features&gt;(@std).features, feature)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_set"></a>

## Function `set`

Helper to include or exclude a feature flag.


<pre><code>fun set(features: &amp;mut vector&lt;u8&gt;, feature: u64, include: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun set(features: &amp;mut vector&lt;u8&gt;, feature: u64, include: bool) &#123;<br/>    let byte_index &#61; feature / 8;<br/>    let bit_mask &#61; 1 &lt;&lt; ((feature % 8) as u8);<br/>    while (vector::length(features) &lt;&#61; byte_index) &#123;<br/>        vector::push_back(features, 0)<br/>    &#125;;<br/>    let entry &#61; vector::borrow_mut(features, byte_index);<br/>    if (include)<br/>        &#42;entry &#61; &#42;entry &#124; bit_mask<br/>    else<br/>        &#42;entry &#61; &#42;entry &amp; (0xff ^ bit_mask)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_contains"></a>

## Function `contains`

Helper to check whether a feature flag is enabled.


<pre><code>fun contains(features: &amp;vector&lt;u8&gt;, feature: u64): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun contains(features: &amp;vector&lt;u8&gt;, feature: u64): bool &#123;<br/>    let byte_index &#61; feature / 8;<br/>    let bit_mask &#61; 1 &lt;&lt; ((feature % 8) as u8);<br/>    byte_index &lt; vector::length(features) &amp;&amp; (&#42;vector::borrow(features, byte_index) &amp; bit_mask) !&#61; 0<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_apply_diff"></a>

## Function `apply_diff`



<pre><code>fun apply_diff(features: &amp;mut vector&lt;u8&gt;, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun apply_diff(features: &amp;mut vector&lt;u8&gt;, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;) &#123;<br/>    vector::for_each(enable, &#124;feature&#124; &#123;<br/>        set(features, feature, true);<br/>    &#125;);<br/>    vector::for_each(disable, &#124;feature&#124; &#123;<br/>        set(features, feature, false);<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_ensure_framework_signer"></a>

## Function `ensure_framework_signer`



<pre><code>fun ensure_framework_signer(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun ensure_framework_signer(account: &amp;signer) &#123;<br/>    let addr &#61; signer::address_of(account);<br/>    assert!(addr &#61;&#61; @std, error::permission_denied(EFRAMEWORK_SIGNER_NEEDED));<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_features_change_feature_flags_for_verification"></a>

## Function `change_feature_flags_for_verification`



<pre><code>&#35;[verify_only]<br/>public fun change_feature_flags_for_verification(framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun change_feature_flags_for_verification(<br/>    framework: &amp;signer,<br/>    enable: vector&lt;u64&gt;,<br/>    disable: vector&lt;u64&gt;<br/>) acquires Features &#123;<br/>    change_feature_flags_internal(framework, enable, disable)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification


<a id="@Specification_1_Features"></a>

### Resource `Features`


<pre><code>struct Features has key<br/></code></pre>



<dl>
<dt>
<code>features: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>pragma bv&#61;b&quot;0&quot;;<br/></code></pre>



<a id="@Specification_1_PendingFeatures"></a>

### Resource `PendingFeatures`


<pre><code>struct PendingFeatures has key<br/></code></pre>



<dl>
<dt>
<code>features: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>



<pre><code>pragma bv&#61;b&quot;0&quot;;<br/></code></pre>



<a id="@Specification_1_periodical_reward_rate_decrease_enabled"></a>

### Function `periodical_reward_rate_decrease_enabled`


<pre><code>public fun periodical_reward_rate_decrease_enabled(): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_periodical_reward_rate_decrease_enabled();<br/></code></pre>




<a id="0x1_features_spec_partial_governance_voting_enabled"></a>


<pre><code>fun spec_partial_governance_voting_enabled(): bool &#123;<br/>   spec_is_enabled(PARTIAL_GOVERNANCE_VOTING)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_partial_governance_voting_enabled"></a>

### Function `partial_governance_voting_enabled`


<pre><code>public fun partial_governance_voting_enabled(): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_partial_governance_voting_enabled();<br/></code></pre>



<a id="@Specification_1_module_event_enabled"></a>

### Function `module_event_enabled`


<pre><code>public fun module_event_enabled(): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_module_event_enabled();<br/></code></pre>



<a id="@Specification_1_change_feature_flags_internal"></a>

### Function `change_feature_flags_internal`


<pre><code>fun change_feature_flags_internal(framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)<br/></code></pre>




<pre><code>pragma opaque;<br/>modifies global&lt;Features&gt;(@std);<br/>aborts_if signer::address_of(framework) !&#61; @std;<br/></code></pre>



<a id="@Specification_1_change_feature_flags_for_next_epoch"></a>

### Function `change_feature_flags_for_next_epoch`


<pre><code>public fun change_feature_flags_for_next_epoch(framework: &amp;signer, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)<br/></code></pre>




<pre><code>aborts_if signer::address_of(framework) !&#61; @std;<br/>pragma opaque;<br/>modifies global&lt;Features&gt;(@std);<br/>modifies global&lt;PendingFeatures&gt;(@std);<br/></code></pre>




<a id="0x1_features_spec_contains"></a>


<pre><code>fun spec_contains(features: vector&lt;u8&gt;, feature: u64): bool &#123;<br/>   ((int2bv((((1 as u8) &lt;&lt; ((feature % (8 as u64)) as u64)) as u8)) as u8) &amp; features[feature/8] as u8) &gt; (0 as u8)<br/>       &amp;&amp; (feature / 8) &lt; len(features)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public fun on_new_epoch(framework: &amp;signer)<br/></code></pre>




<pre><code>requires @std &#61;&#61; signer::address_of(framework);<br/>let features_pending &#61; global&lt;PendingFeatures&gt;(@std).features;<br/>let post features_std &#61; global&lt;Features&gt;(@std).features;<br/>ensures exists&lt;PendingFeatures&gt;(@std) &#61;&#61;&gt; features_std &#61;&#61; features_pending;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_is_enabled"></a>

### Function `is_enabled`


<pre><code>&#35;[view]<br/>public fun is_enabled(feature: u64): bool<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_is_enabled(feature);<br/></code></pre>




<a id="0x1_features_spec_is_enabled"></a>


<pre><code>fun spec_is_enabled(feature: u64): bool;<br/></code></pre>




<a id="0x1_features_spec_periodical_reward_rate_decrease_enabled"></a>


<pre><code>fun spec_periodical_reward_rate_decrease_enabled(): bool &#123;<br/>   spec_is_enabled(PERIODICAL_REWARD_RATE_DECREASE)<br/>&#125;<br/></code></pre>




<a id="0x1_features_spec_fee_payer_enabled"></a>


<pre><code>fun spec_fee_payer_enabled(): bool &#123;<br/>   spec_is_enabled(FEE_PAYER_ENABLED)<br/>&#125;<br/></code></pre>




<a id="0x1_features_spec_collect_and_distribute_gas_fees_enabled"></a>


<pre><code>fun spec_collect_and_distribute_gas_fees_enabled(): bool &#123;<br/>   spec_is_enabled(COLLECT_AND_DISTRIBUTE_GAS_FEES)<br/>&#125;<br/></code></pre>




<a id="0x1_features_spec_module_event_enabled"></a>


<pre><code>fun spec_module_event_enabled(): bool &#123;<br/>   spec_is_enabled(MODULE_EVENT)<br/>&#125;<br/></code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code>fun set(features: &amp;mut vector&lt;u8&gt;, feature: u64, include: bool)<br/></code></pre>




<pre><code>pragma bv&#61;b&quot;0&quot;;<br/>aborts_if false;<br/>ensures feature / 8 &lt; len(features);<br/>ensures include &#61;&#61; spec_contains(features, feature);<br/></code></pre>



<a id="@Specification_1_contains"></a>

### Function `contains`


<pre><code>fun contains(features: &amp;vector&lt;u8&gt;, feature: u64): bool<br/></code></pre>




<pre><code>pragma bv&#61;b&quot;0&quot;;<br/>aborts_if false;<br/>ensures result &#61;&#61; spec_contains(features, feature);<br/></code></pre>



<a id="@Specification_1_apply_diff"></a>

### Function `apply_diff`


<pre><code>fun apply_diff(features: &amp;mut vector&lt;u8&gt;, enable: vector&lt;u64&gt;, disable: vector&lt;u64&gt;)<br/></code></pre>




<pre><code>aborts_if [abstract] false;<br/>ensures [abstract] forall i in disable: !spec_contains(features, i);<br/>ensures [abstract] forall i in enable: !vector::spec_contains(disable, i)<br/>    &#61;&#61;&gt; spec_contains(features, i);<br/>pragma opaque;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
