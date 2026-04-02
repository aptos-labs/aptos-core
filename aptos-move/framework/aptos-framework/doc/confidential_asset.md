
<a id="0x1_confidential_asset"></a>

# Module `0x1::confidential_asset`

Confidential Asset (CA) Standard: privacy-focused fungible asset transfers with obfuscated amounts.


-  [Enum `AuditorConfig`](#0x1_confidential_asset_AuditorConfig)
-  [Enum `EffectiveAuditorConfig`](#0x1_confidential_asset_EffectiveAuditorConfig)
-  [Enum `EffectiveAuditorHint`](#0x1_confidential_asset_EffectiveAuditorHint)
-  [Enum Resource `GlobalConfig`](#0x1_confidential_asset_GlobalConfig)
-  [Enum Resource `AssetConfig`](#0x1_confidential_asset_AssetConfig)
-  [Enum Resource `ConfidentialStore`](#0x1_confidential_asset_ConfidentialStore)
-  [Enum `Registered`](#0x1_confidential_asset_Registered)
-  [Enum `Deposited`](#0x1_confidential_asset_Deposited)
-  [Enum `Withdrawn`](#0x1_confidential_asset_Withdrawn)
-  [Enum `Transferred`](#0x1_confidential_asset_Transferred)
-  [Enum `Normalized`](#0x1_confidential_asset_Normalized)
-  [Enum `RolledOver`](#0x1_confidential_asset_RolledOver)
-  [Enum `KeyRotated`](#0x1_confidential_asset_KeyRotated)
-  [Enum `IncomingTransfersPauseChanged`](#0x1_confidential_asset_IncomingTransfersPauseChanged)
-  [Enum `AllowListingChanged`](#0x1_confidential_asset_AllowListingChanged)
-  [Enum `ConfidentialityForAssetTypeChanged`](#0x1_confidential_asset_ConfidentialityForAssetTypeChanged)
-  [Enum `GlobalAuditorChanged`](#0x1_confidential_asset_GlobalAuditorChanged)
-  [Enum `AssetSpecificAuditorChanged`](#0x1_confidential_asset_AssetSpecificAuditorChanged)
-  [Enum `RegistrationProof`](#0x1_confidential_asset_RegistrationProof)
-  [Enum `WithdrawalProof`](#0x1_confidential_asset_WithdrawalProof)
-  [Enum `TransferProof`](#0x1_confidential_asset_TransferProof)
-  [Enum `KeyRotationProof`](#0x1_confidential_asset_KeyRotationProof)
-  [Constants](#@Constants_0)
    -  [[test_only] The confidential asset module initialization failed.](#@[test_only]_The_confidential_asset_module_initialization_failed._1)
-  [Function `init_module`](#0x1_confidential_asset_init_module)
-  [Function `init_module_for_devnet`](#0x1_confidential_asset_init_module_for_devnet)
-  [Function `register_raw`](#0x1_confidential_asset_register_raw)
-  [Function `is_safe_for_confidentiality`](#0x1_confidential_asset_is_safe_for_confidentiality)
-  [Function `register`](#0x1_confidential_asset_register)
-  [Function `deposit`](#0x1_confidential_asset_deposit)
-  [Function `withdraw_to_raw`](#0x1_confidential_asset_withdraw_to_raw)
-  [Function `withdraw_to`](#0x1_confidential_asset_withdraw_to)
-  [Function `confidential_transfer_raw`](#0x1_confidential_asset_confidential_transfer_raw)
-  [Function `confidential_transfer`](#0x1_confidential_asset_confidential_transfer)
-  [Function `rotate_encryption_key_raw`](#0x1_confidential_asset_rotate_encryption_key_raw)
-  [Function `rotate_encryption_key`](#0x1_confidential_asset_rotate_encryption_key)
-  [Function `normalize_raw`](#0x1_confidential_asset_normalize_raw)
-  [Function `normalize`](#0x1_confidential_asset_normalize)
-  [Function `rollover_pending_balance`](#0x1_confidential_asset_rollover_pending_balance)
-  [Function `rollover_pending_balance_and_pause`](#0x1_confidential_asset_rollover_pending_balance_and_pause)
-  [Function `set_incoming_transfers_paused`](#0x1_confidential_asset_set_incoming_transfers_paused)
-  [Function `set_allow_listing`](#0x1_confidential_asset_set_allow_listing)
-  [Function `set_confidentiality_for_apt`](#0x1_confidential_asset_set_confidentiality_for_apt)
-  [Function `set_confidentiality_for_asset_type`](#0x1_confidential_asset_set_confidentiality_for_asset_type)
-  [Function `set_asset_specific_auditor`](#0x1_confidential_asset_set_asset_specific_auditor)
-  [Function `set_global_auditor`](#0x1_confidential_asset_set_global_auditor)
-  [Function `update_auditor`](#0x1_confidential_asset_update_auditor)
-  [Function `has_confidential_store`](#0x1_confidential_asset_has_confidential_store)
-  [Function `is_confidentiality_enabled_for_asset_type`](#0x1_confidential_asset_is_confidentiality_enabled_for_asset_type)
-  [Function `is_allow_listing_required`](#0x1_confidential_asset_is_allow_listing_required)
-  [Function `get_pending_balance`](#0x1_confidential_asset_get_pending_balance)
-  [Function `get_available_balance`](#0x1_confidential_asset_get_available_balance)
-  [Function `get_encryption_key`](#0x1_confidential_asset_get_encryption_key)
-  [Function `is_normalized`](#0x1_confidential_asset_is_normalized)
-  [Function `incoming_transfers_paused`](#0x1_confidential_asset_incoming_transfers_paused)
-  [Function `get_effective_auditor_hint`](#0x1_confidential_asset_get_effective_auditor_hint)
-  [Function `get_asset_specific_auditor_config`](#0x1_confidential_asset_get_asset_specific_auditor_config)
-  [Function `get_global_auditor_config`](#0x1_confidential_asset_get_global_auditor_config)
-  [Function `get_effective_auditor_config`](#0x1_confidential_asset_get_effective_auditor_config)
-  [Function `get_total_confidential_supply`](#0x1_confidential_asset_get_total_confidential_supply)
-  [Function `get_num_transfers_received`](#0x1_confidential_asset_get_num_transfers_received)
-  [Function `get_max_transfers_before_rollover`](#0x1_confidential_asset_get_max_transfers_before_rollover)
-  [Function `update_auditor_hint`](#0x1_confidential_asset_update_auditor_hint)
-  [Function `get_asset_config_address`](#0x1_confidential_asset_get_asset_config_address)
-  [Function `get_asset_config_address_or_create`](#0x1_confidential_asset_get_asset_config_address_or_create)
-  [Function `get_global_config_signer`](#0x1_confidential_asset_get_global_config_signer)
-  [Function `get_pool_fa_store`](#0x1_confidential_asset_get_pool_fa_store)
-  [Function `ensure_pool_fa_store`](#0x1_confidential_asset_ensure_pool_fa_store)
-  [Function `get_confidential_store_signer`](#0x1_confidential_asset_get_confidential_store_signer)
-  [Function `get_confidential_store_address`](#0x1_confidential_asset_get_confidential_store_address)
-  [Function `borrow_confidential_store`](#0x1_confidential_asset_borrow_confidential_store)
-  [Function `borrow_confidential_store_mut`](#0x1_confidential_asset_borrow_confidential_store_mut)
-  [Function `get_asset_config_signer`](#0x1_confidential_asset_get_asset_config_signer)
-  [Function `construct_confidential_store_seed`](#0x1_confidential_asset_construct_confidential_store_seed)
-  [Function `construct_asset_config_seed`](#0x1_confidential_asset_construct_asset_config_seed)
-  [Function `assert_valid_registration_proof`](#0x1_confidential_asset_assert_valid_registration_proof)
-  [Function `assert_valid_withdrawal_proof`](#0x1_confidential_asset_assert_valid_withdrawal_proof)
-  [Function `assert_valid_transfer_proof`](#0x1_confidential_asset_assert_valid_transfer_proof)
-  [Function `assert_valid_key_rotation_proof`](#0x1_confidential_asset_assert_valid_key_rotation_proof)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="confidential_amount.md#0x1_confidential_amount">0x1::confidential_amount</a>;
<b>use</b> <a href="confidential_balance.md#0x1_confidential_balance">0x1::confidential_balance</a>;
<b>use</b> <a href="confidential_range_proofs.md#0x1_confidential_range_proofs">0x1::confidential_range_proofs</a>;
<b>use</b> <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">0x1::dispatchable_fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs">0x1::ristretto255_bulletproofs</a>;
<b>use</b> <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation">0x1::sigma_protocol_key_rotation</a>;
<b>use</b> <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof">0x1::sigma_protocol_proof</a>;
<b>use</b> <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration">0x1::sigma_protocol_registration</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x1_sigma_protocol_statement">0x1::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer">0x1::sigma_protocol_transfer</a>;
<b>use</b> <a href="sigma_protocol_utils.md#0x1_sigma_protocol_utils">0x1::sigma_protocol_utils</a>;
<b>use</b> <a href="sigma_protocol_withdraw.md#0x1_sigma_protocol_withdraw">0x1::sigma_protocol_withdraw</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_confidential_asset_AuditorConfig"></a>

## Enum `AuditorConfig`

Bundles an auditor's encryption key with its epoch counter (both always modified together).


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">AuditorConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ek: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch: u64</code>
</dt>
<dd>
 Tracks how many times the auditor EK has been installed or changed (not removed). Starts at 0, indicating
 no auditor was ever installed. Increments each time a new EK is set (None → Some(ek) or Some(old) → Some(new)).
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_EffectiveAuditorConfig"></a>

## Enum `EffectiveAuditorConfig`

When developers fetch the effective auditor config, we wrap it in this struct to indicate whether they've fetched the global or the asset-specific auditor config


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorConfig">EffectiveAuditorConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>is_global: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>config: <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_EffectiveAuditorHint"></a>

## Enum `EffectiveAuditorHint`

When auditors fetch the effective auditor epoch from a <code><a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a></code>, they need both the <code>epoch</code> number and the <code>is_global</code> flag to tell if the auditor ciphertext is stale


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorHint">EffectiveAuditorHint</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>is_global: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_GlobalConfig"></a>

## Enum Resource `GlobalConfig`

Global configuration for the confidential asset protocol, installed during <code>init_module</code>.


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allow_list_enabled: bool</code>
</dt>
<dd>
 Indicates whether the allow list is enabled. If <code><b>true</b></code>, only asset types from the allow list can be transferred.
 This flag is managed by the governance module.
</dd>
<dt>
<code>global_auditor: <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a></code>
</dt>
<dd>
 The global auditor. Asset-specific auditors take precedence.
</dd>
<dt>
<code>extend_ref: <a href="object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>
 Used to derive a signer that owns all the FAs' primary stores and <code><a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a></code> objects.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_AssetConfig"></a>

## Enum Resource `AssetConfig`

Per-asset-type configuration (allow-listing, auditor).


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allowed: bool</code>
</dt>
<dd>
 Indicates whether the asset type is allowed for confidential transfers, can be toggled by the governance
 module. Withdrawals are always allowed, even when this is set to <code><b>false</b></code>.
 If <code>GlobalConfig::allow_list_enabled</code> is <code><b>false</b></code>, all asset types are allowed, even if this is <code><b>false</b></code>.
</dd>
<dt>
<code>auditor: <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a></code>
</dt>
<dd>
 The asset-specific auditor. Takes precedence over the global auditor.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_ConfidentialStore"></a>

## Enum Resource `ConfidentialStore`

Per-(user, asset-type) encrypted balance store (confidential variant of <code>FungibleStore</code>).


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pause_incoming: bool</code>
</dt>
<dd>
 Must be paused before key rotation to prevent mid-rotation pending balance changes.
</dd>
<dt>
<code>normalized: bool</code>
</dt>
<dd>
 True if all available balance chunks are within 16-bit bounds (required before rollover).
</dd>
<dt>
<code>transfers_received: u64</code>
</dt>
<dd>
 Number of transfers received; upper-bounds pending balance chunk sizes.
</dd>
<dt>
<code>pending_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;</code>
</dt>
<dd>
 Incoming transfers accumulate here; must be rolled over into <code>available_balance</code> to spend.
</dd>
<dt>
<code>available_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>
 Spendable balance (8 chunks, 128-bit). R_aud components for auditor decryption (empty if no auditor).
</dd>
<dt>
<code>ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>
 User's encryption key for this asset type.
</dd>
<dt>
<code>auditor_hint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorHint">confidential_asset::EffectiveAuditorHint</a>&gt;</code>
</dt>
<dd>
 Tracks which auditor the balance ciphertext is encrypted for: global/effective and epoch
</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_Registered"></a>

## Enum `Registered`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_Registered">Registered</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_Deposited"></a>

## Enum `Deposited`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_Deposited">Deposited</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_pending_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_Withdrawn"></a>

## Enum `Withdrawn`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_Withdrawn">Withdrawn</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>from: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>amount: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_available_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>auditor_hint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorHint">confidential_asset::EffectiveAuditorHint</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_Transferred"></a>

## Enum `Transferred`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_Transferred">Transferred</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>from: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><b>to</b>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>amount: <a href="confidential_amount.md#0x1_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a></code>
</dt>
<dd>

</dd>
<dt>
<code>ek_volun_auds: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sender_auditor_hint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorHint">confidential_asset::EffectiveAuditorHint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_sender_available_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_recip_pending_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>memo: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_Normalized"></a>

## Enum `Normalized`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_Normalized">Normalized</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_available_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>auditor_hint: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorHint">confidential_asset::EffectiveAuditorHint</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_RolledOver"></a>

## Enum `RolledOver`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_RolledOver">RolledOver</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_available_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_KeyRotated"></a>

## Enum `KeyRotated`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_KeyRotated">KeyRotated</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_available_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_IncomingTransfersPauseChanged"></a>

## Enum `IncomingTransfersPauseChanged`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_IncomingTransfersPauseChanged">IncomingTransfersPauseChanged</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>addr: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>paused: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_AllowListingChanged"></a>

## Enum `AllowListingChanged`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_AllowListingChanged">AllowListingChanged</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>enabled: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_ConfidentialityForAssetTypeChanged"></a>

## Enum `ConfidentialityForAssetTypeChanged`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialityForAssetTypeChanged">ConfidentialityForAssetTypeChanged</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>allowed: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_GlobalAuditorChanged"></a>

## Enum `GlobalAuditorChanged`

SDK note: when you see this event, call <code>get_effective_auditor</code> to determine the current effective EK
for any asset that doesn't have an asset-specific auditor override.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_GlobalAuditorChanged">GlobalAuditorChanged</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>new: <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_AssetSpecificAuditorChanged"></a>

## Enum `AssetSpecificAuditorChanged`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x1_confidential_asset_AssetSpecificAuditorChanged">AssetSpecificAuditorChanged</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new: <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_RegistrationProof"></a>

## Enum `RegistrationProof`

Proof of knowledge of DK for registration: $\Sigma$-protocol proving $H = \mathsf{dk} \cdot \mathsf{ek}$.


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_RegistrationProof">RegistrationProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_WithdrawalProof"></a>

## Enum `WithdrawalProof`

Withdrawal proof: new normalized balance, range proof, and $\Sigma$-protocol for $\mathcal{R}^{-}_\mathsf{withdraw}$.


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_WithdrawalProof">WithdrawalProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>compressed_new_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>

</dd>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_TransferProof"></a>

## Enum `TransferProof`

Transfer proof: new balance, encrypted amount, range proofs, and $\Sigma$-protocol for $\mathcal{R}^{-}_\mathsf{txfer}$.


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_TransferProof">TransferProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>compressed_new_balance: <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_amount: <a href="confidential_amount.md#0x1_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a></code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_ek_volun_auds: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>

</dd>
<dt>
<code>zkrp_amount: <a href="../../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>

</dd>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x1_confidential_asset_KeyRotationProof"></a>

## Enum `KeyRotationProof`

Key rotation proof: new EK, re-encrypted R components, and $\Sigma$-protocol for correct re-encryption.


<pre><code>enum <a href="confidential_asset.md#0x1_confidential_asset_KeyRotationProof">KeyRotationProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>compressed_new_ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_new_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_confidential_asset_E_AUDITOR_COUNT_MISMATCH"></a>

The number of auditor R components in the proof does not match the expected auditor count.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>: u64 = 12;
</code></pre>



<a id="0x1_confidential_asset_E_ALLOW_LISTING_IS_DISABLED"></a>

Allow listing is not enabled yet.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_ALLOW_LISTING_IS_DISABLED">E_ALLOW_LISTING_IS_DISABLED</a>: u64 = 17;
</code></pre>



<a id="0x1_confidential_asset_E_ALREADY_NORMALIZED"></a>

The balance is already normalized and cannot be normalized again.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_ALREADY_NORMALIZED">E_ALREADY_NORMALIZED</a>: u64 = 8;
</code></pre>



<a id="0x1_confidential_asset_E_ASSET_TYPE_DISALLOWED"></a>

The asset type is currently not allowed for confidential transfers.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>: u64 = 9;
</code></pre>



<a id="0x1_confidential_asset_E_AUDITOR_EK_IS_IDENTITY"></a>

The auditor encryption key must not be the identity (zero) point.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_AUDITOR_EK_IS_IDENTITY">E_AUDITOR_EK_IS_IDENTITY</a>: u64 = 14;
</code></pre>



<a id="0x1_confidential_asset_E_CONFIDENTIAL_STORE_ALREADY_REGISTERED"></a>

The confidential store has already been published for the given user and asset-type pair: user need not call <code>register</code> again.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_CONFIDENTIAL_STORE_ALREADY_REGISTERED">E_CONFIDENTIAL_STORE_ALREADY_REGISTERED</a>: u64 = 2;
</code></pre>



<a id="0x1_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED"></a>

The confidential store has not been published for the given user and asset-type pair: user should call <code>register</code>.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>: u64 = 3;
</code></pre>



<a id="0x1_confidential_asset_E_INCOMING_TRANSFERS_NOT_PAUSED"></a>

Incoming transfers must be paused before key rotation.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_INCOMING_TRANSFERS_NOT_PAUSED">E_INCOMING_TRANSFERS_NOT_PAUSED</a>: u64 = 10;
</code></pre>



<a id="0x1_confidential_asset_E_INCOMING_TRANSFERS_PAUSED"></a>

Incoming transfers must NOT be paused before depositing or receiving a transfer.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>: u64 = 4;
</code></pre>



<a id="0x1_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET"></a>


<a id="@[test_only]_The_confidential_asset_module_initialization_failed._1"></a>

### [test_only] The confidential asset module initialization failed.



<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>: u64 = 1000;
</code></pre>



<a id="0x1_confidential_asset_E_INTERNAL_ERROR"></a>

An internal error occurred: there is either a bug or a misconfiguration in the contract.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>: u64 = 999;
</code></pre>



<a id="0x1_confidential_asset_E_NORMALIZATION_REQUIRED"></a>

The available balance must be normalized before roll-over to ensure available balance chunks remain 32-bit after.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_NORMALIZATION_REQUIRED">E_NORMALIZATION_REQUIRED</a>: u64 = 7;
</code></pre>



<a id="0x1_confidential_asset_E_NOTHING_TO_ROLLOVER"></a>

There are no pending transfers to roll over.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_NOTHING_TO_ROLLOVER">E_NOTHING_TO_ROLLOVER</a>: u64 = 13;
</code></pre>



<a id="0x1_confidential_asset_E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE"></a>

No user has deposited this asset type yet into their confidential store.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE">E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE</a>: u64 = 11;
</code></pre>



<a id="0x1_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER"></a>

The receiver's pending balance has accumulated too many incoming transfers and must be rolled over into the available balance.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>: u64 = 6;
</code></pre>



<a id="0x1_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION"></a>

The pending balance must be zero before rotating the encryption key.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>: u64 = 5;
</code></pre>



<a id="0x1_confidential_asset_E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE"></a>

The range proof system does not support sufficient range.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>: u64 = 1;
</code></pre>



<a id="0x1_confidential_asset_E_SELF_TRANSFER"></a>

Self-transfers are not allowed: sender and recipient must be different.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_SELF_TRANSFER">E_SELF_TRANSFER</a>: u64 = 15;
</code></pre>



<a id="0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA"></a>

"Dispatchable" fungible asset types whose withdraw, deposit, balance or supply functions can be customized are not supported, for now.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA">E_UNSAFE_DISPATCHABLE_FA</a>: u64 = 16;
</code></pre>



<a id="0x1_confidential_asset_MAINNET_CHAIN_ID"></a>

The mainnet chain ID. If the chain ID is 1, the allow list is enabled.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_MAINNET_CHAIN_ID">MAINNET_CHAIN_ID</a>: u8 = 1;
</code></pre>



<a id="0x1_confidential_asset_MAX_NUM_BITS_IN_SCALAR_FIELD"></a>

Any natural number that fits in this # of bits will be less than the Ristretto255 order $p$ and thus fit in its scalar field $\mathbb{Z}_p$ without "wrapping around."


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_MAX_NUM_BITS_IN_SCALAR_FIELD">MAX_NUM_BITS_IN_SCALAR_FIELD</a>: u64 = 252;
</code></pre>



<a id="0x1_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER"></a>

The maximum number of transactions can be aggregated on the pending balance before rollover is required.
i.e., <code>ConfidentialStore::transfers_received</code> will never exceed this value.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>: u64 = 65536;
</code></pre>



<a id="0x1_confidential_asset_TESTNET_CHAIN_ID"></a>

The testnet chain ID.


<pre><code><b>const</b> <a href="confidential_asset.md#0x1_confidential_asset_TESTNET_CHAIN_ID">TESTNET_CHAIN_ID</a>: u8 = 2;
</code></pre>



<a id="0x1_confidential_asset_init_module"></a>

## Function `init_module`

Called once when this module is first published on-chain.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_init_module">init_module</a>(deployer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_init_module">init_module</a>(deployer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    // This is me being overly cautious: I added it <b>to</b> double-check my understanding that the VM always passes
    // the publishing <a href="account.md#0x1_account">account</a> <b>as</b> deployer. It does, so the <b>assert</b> is redundant (it can never fail).
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer) == @aptos_framework, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>));
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/math64.md#0x1_math64_pow">math64::pow</a>(2, get_chunk_size_bits()) == get_chunk_upper_bound(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>));
    <b>assert</b>!(
        bulletproofs::get_max_range_bits() &gt;= <a href="confidential_balance.md#0x1_confidential_balance_get_chunk_size_bits">confidential_balance::get_chunk_size_bits</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>)
    );

    // Available must have more chunks than pending (rollover safety)
    <b>let</b> num_avail_chunks = <a href="confidential_balance.md#0x1_confidential_balance_get_num_available_chunks">confidential_balance::get_num_available_chunks</a>();
    <b>let</b> num_pend_chunks = <a href="confidential_balance.md#0x1_confidential_balance_get_num_pending_chunks">confidential_balance::get_num_pending_chunks</a>();
    <b>assert</b>!(num_avail_chunks &gt;= num_pend_chunks);

    // Available balance chunking must be done so that <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> balance is representable <b>as</b> a Scalar, w/o wrap-around
    <b>let</b> avail_balance_upper_bound = get_chunk_size_bits() * num_avail_chunks;
    // FA balances <b>use</b> u128 amounts
    <b>assert</b>!(avail_balance_upper_bound == 128);
    // no modular wraparound on available balances
    <b>assert</b>!(avail_balance_upper_bound &lt;= <a href="confidential_asset.md#0x1_confidential_asset_MAX_NUM_BITS_IN_SCALAR_FIELD">MAX_NUM_BITS_IN_SCALAR_FIELD</a>);

    // Pending balance chunking must be done so that <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> balance is representable <b>as</b> a Scalar, w/o wrap-around
    <b>let</b> pend_balance_upper_bound = get_chunk_size_bits() * num_pend_chunks;
    // FA deposit/withdraw <b>use</b> u64 amounts
    <b>assert</b>!(pend_balance_upper_bound == 64);
    // no modular wraparound on pending balances nor transferred amounts
    <b>assert</b>!(pend_balance_upper_bound &lt;= <a href="confidential_asset.md#0x1_confidential_asset_MAX_NUM_BITS_IN_SCALAR_FIELD">MAX_NUM_BITS_IN_SCALAR_FIELD</a>);

    <b>let</b> deployer_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer);
    <b>let</b> <a href="chain_id.md#0x1_chain_id">chain_id</a> = <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>();
    <b>move_to</b>(
        deployer,
        GlobalConfig::V1 {
            allow_list_enabled: <a href="chain_id.md#0x1_chain_id">chain_id</a> == <a href="confidential_asset.md#0x1_confidential_asset_MAINNET_CHAIN_ID">MAINNET_CHAIN_ID</a> || <a href="chain_id.md#0x1_chain_id">chain_id</a> == <a href="confidential_asset.md#0x1_confidential_asset_TESTNET_CHAIN_ID">TESTNET_CHAIN_ID</a>,
            global_auditor: AuditorConfig::V1 { ek: std::option::none(), epoch: 0 },
            // DO NOT CHANGE: using long syntax until framework change is released <b>to</b> mainnet
            extend_ref: <a href="object.md#0x1_object_create_object">object::create_object</a>(deployer_address).generate_extend_ref()
        }
    );
}
</code></pre>



</details>

<a id="0x1_confidential_asset_init_module_for_devnet"></a>

## Function `init_module_for_devnet`

Initializes the module for devnet/tests. Asserts non-mainnet, non-testnet chain.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_init_module_for_devnet">init_module_for_devnet</a>(deployer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_init_module_for_devnet">init_module_for_devnet</a>(deployer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer) == @aptos_framework,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>)
    );
    <b>assert</b>!(
        <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() != <a href="confidential_asset.md#0x1_confidential_asset_MAINNET_CHAIN_ID">MAINNET_CHAIN_ID</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>)
    );
    <b>assert</b>!(
        <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() != <a href="confidential_asset.md#0x1_confidential_asset_TESTNET_CHAIN_ID">TESTNET_CHAIN_ID</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>)
    );

    <a href="confidential_asset.md#0x1_confidential_asset_init_module">init_module</a>(deployer)
}
</code></pre>



</details>

<a id="0x1_confidential_asset_register_raw"></a>

## Function `register_raw`

Deserializes cryptographic data and forwards to <code>register</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_register_raw">register_raw</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_register_raw">register_raw</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>let</b> ek = new_compressed_point_from_bytes(ek).extract();
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(sigma_proto_comm, sigma_proto_resp);
    <b>let</b> proof = RegistrationProof::V1 { sigma };

    <a href="confidential_asset.md#0x1_confidential_asset_register">register</a>(sender, asset_type, ek, proof);
}
</code></pre>



</details>

<a id="0x1_confidential_asset_is_safe_for_confidentiality"></a>

## Function `is_safe_for_confidentiality`

Dispatchable fungible asset (DFA) types can, for example, dynamically change user balances upon a call to
<code><a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>()</code>, based say, on a multiplier. We do not yet see how to *generically* handle such
dynamic behavior in a confidential context, where balances are encrypted on-chain and cannot be modified in
arbitrary ways. Similarly, we also forbid "total supply" dispatch functions, out of an abundance of caution.

Furthermore, even for DFAs that only have custom "withdraw/deposit" dispatch functions, it is unclear how to
*generically* support any such functionality. As a result, for now we only support non-dispatchable (vanilla)
fungible asset (FA) types.

For example, sender blocklists implemented via "withdraw" dispatching would only be enforced when users veil/
unveil their tokens into/from the confidential asset pool. (This is because a <code>confidential_transfer</code> cannot,
by definition, interact with any (D)FA functions, or it would be forced to leak amounts/balances). In the future,
we could add support for dispatch functions that only look at the sender's address (and not at the amount/
balances). This way, we could *generically* handle them here, given they are implemented in a type-safe way that
allows us to check they are enabled.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_safe_for_confidentiality">is_safe_for_confidentiality</a>(asset_type: &<a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_safe_for_confidentiality">is_safe_for_confidentiality</a>(asset_type: &Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool {
    !<a href="fungible_asset.md#0x1_fungible_asset_is_asset_type_dispatchable">fungible_asset::is_asset_type_dispatchable</a>(*asset_type)
}
</code></pre>



</details>

<a id="0x1_confidential_asset_register"></a>

## Function `register`

Registers a confidential store for a specified asset type, encrypted under the given EK.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_register">register</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, proof: <a href="confidential_asset.md#0x1_confidential_asset_RegistrationProof">confidential_asset::RegistrationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_register">register</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: CompressedRistretto,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_RegistrationProof">RegistrationProof</a>
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_safe_for_confidentiality">is_safe_for_confidentiality</a>(&asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA">E_UNSAFE_DISPATCHABLE_FA</a>));
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));

    <b>assert</b>!(
        !<a href="confidential_asset.md#0x1_confidential_asset_has_confidential_store">has_confidential_store</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), asset_type),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_CONFIDENTIAL_STORE_ALREADY_REGISTERED">E_CONFIDENTIAL_STORE_ALREADY_REGISTERED</a>)
    );

    // Makes sure the user knows their <a href="decryption.md#0x1_decryption">decryption</a> key.
    <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_registration_proof">assert_valid_registration_proof</a>(sender, asset_type, &ek, proof);

    <b>let</b> ca_store = ConfidentialStore::V1 {
        pause_incoming: <b>false</b>,
        normalized: <b>true</b>,
        transfers_received: 0,
        pending_balance: new_zero_pending_compressed(),
        available_balance: new_zero_available_compressed(),
        ek,
        auditor_hint: std::option::none() // balance == 0 is publicly-known ==&gt; auditor ciphertext is left empty
    };

    <b>move_to</b>(&<a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_signer">get_confidential_store_signer</a>(sender, asset_type), ca_store);
    <a href="event.md#0x1_event_emit">event::emit</a>(Registered::V1 { addr: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), asset_type, ek });
}
</code></pre>



</details>

<a id="0x1_confidential_asset_deposit"></a>

## Function `deposit`

Deposits tokens from the sender's primary FA store into their pending balance.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_deposit">deposit</a>(depositor: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_deposit">deposit</a>(
    depositor: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    amount: u64
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(depositor);

    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_safe_for_confidentiality">is_safe_for_confidentiality</a>(&asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA">E_UNSAFE_DISPATCHABLE_FA</a>));
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));
    <b>assert</b>!(!<a href="confidential_asset.md#0x1_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(addr, asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>));

    // Note: Gets the "confidential asset pool" for this asset type, or sets it up <b>if</b> this asset type is veiled for the first time
    <b>let</b> pool_fa_store = <a href="confidential_asset.md#0x1_confidential_asset_ensure_pool_fa_store">ensure_pool_fa_store</a>(asset_type);

    // Step 1: Transfer the asset from the user's <a href="account.md#0x1_account">account</a> into the confidential asset pool.
    //
    // Note: Dispatchable transfers may deliver less than `amount` (e.g., due <b>to</b> fees for deflationary tokens), so
    // we measure the pool balance before & after <b>to</b> credit only what was actually received.
    <b>let</b> before = <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(pool_fa_store);
    <b>let</b> depositor_fa_store = <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_fungible_store::primary_store</a>(addr, asset_type);
    <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">dispatchable_fungible_asset::transfer</a>(depositor, depositor_fa_store, pool_fa_store, amount);

    // Step 2: "Mint" corresponding confidential assets for the depositor, and add them <b>to</b> their pending balance.
    <b>let</b> ca_store = <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(addr, asset_type);

    add_assign_pending(&<b>mut</b> ca_store.pending_balance, &new_pending_u64_no_randomness(amount));
    ca_store.transfers_received += 1;

    // Make sure the depositor <b>has</b> "room" in their pending balance for this deposit
    <b>assert</b>!(
        ca_store.transfers_received &lt;= <a href="confidential_asset.md#0x1_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>)
    );

    <a href="event.md#0x1_event_emit">event::emit</a>(Deposited::V1 { addr, amount, asset_type, new_pending_balance: ca_store.pending_balance });

    // Re-asserting dispatchable FA functionality that charges fees on withdraw/deposit was not invoked.
    <b>assert</b>!(amount == <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(pool_fa_store) - before, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA">E_UNSAFE_DISPATCHABLE_FA</a>));
}
</code></pre>



</details>

<a id="0x1_confidential_asset_withdraw_to_raw"></a>

## Function `withdraw_to_raw`

Deserializes cryptographic data and forwards to <code>withdraw_to</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_withdraw_to_raw">withdraw_to_raw</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64, new_balance_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_R_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, zkrp_new_balance: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_withdraw_to_raw">withdraw_to_raw</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64,
    new_balance_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_R_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,  // effective auditor R component
    zkrp_new_balance: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>let</b> compressed_new_balance = new_compressed_available_from_bytes(new_balance_P, new_balance_R, new_balance_R_aud);
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(sigma_proto_comm, sigma_proto_resp);
    <b>let</b> proof = WithdrawalProof::V1 { compressed_new_balance, zkrp_new_balance, sigma };

    <a href="confidential_asset.md#0x1_confidential_asset_withdraw_to">withdraw_to</a>(sender, asset_type, <b>to</b>, amount, proof);
}
</code></pre>



</details>

<a id="0x1_confidential_asset_withdraw_to"></a>

## Function `withdraw_to`

Withdraws tokens from the sender's available balance to recipient's primary FA store. Also used internally by <code>normalize</code> (amount = 0).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_withdraw_to">withdraw_to</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64, proof: <a href="confidential_asset.md#0x1_confidential_asset_WithdrawalProof">confidential_asset::WithdrawalProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_withdraw_to">withdraw_to</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_WithdrawalProof">WithdrawalProof</a>
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_safe_for_confidentiality">is_safe_for_confidentiality</a>(&asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA">E_UNSAFE_DISPATCHABLE_FA</a>));

    <b>let</b> sender_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    // Read values before mutable borrow <b>to</b> avoid conflicting borrows of <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>
    <b>let</b> ek = <a href="confidential_asset.md#0x1_confidential_asset_get_encryption_key">get_encryption_key</a>(sender_addr, asset_type);
    <b>let</b> old_balance = <a href="confidential_asset.md#0x1_confidential_asset_get_available_balance">get_available_balance</a>(sender_addr, asset_type);
    <b>let</b> effective_auditor = <a href="confidential_asset.md#0x1_confidential_asset_get_effective_auditor_config">get_effective_auditor_config</a>(asset_type);

    <b>let</b> compressed_new_balance = <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_withdrawal_proof">assert_valid_withdrawal_proof</a>(
        sender, asset_type,
        &ek, amount, &old_balance, &effective_auditor.config.ek, proof
    );

    <b>let</b> ca_store = <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(sender_addr, asset_type);
    ca_store.normalized = <b>true</b>;
    ca_store.available_balance = compressed_new_balance;
    ca_store.<a href="confidential_asset.md#0x1_confidential_asset_update_auditor_hint">update_auditor_hint</a>(&effective_auditor); // enables auditor <b>to</b> later tell whether their balance ciphertext is stale

    // Copy state for the <a href="event.md#0x1_event">event</a> (before <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> further borrows)
    <b>let</b> new_available_balance = ca_store.available_balance;
    <b>let</b> auditor_hint = ca_store.auditor_hint;

    <b>if</b> (amount &gt; 0) {
        <b>let</b> pool_fa_store = <a href="confidential_asset.md#0x1_confidential_asset_get_pool_fa_store">get_pool_fa_store</a>(asset_type);  // must exist b.c. sender's CA store <b>exists</b>
        <b>let</b> before = <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(pool_fa_store);

        <a href="dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">dispatchable_fungible_asset::transfer</a>(&<a href="confidential_asset.md#0x1_confidential_asset_get_global_config_signer">get_global_config_signer</a>(), pool_fa_store, <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(<b>to</b>, asset_type), amount);
        <a href="event.md#0x1_event_emit">event::emit</a>(Withdrawn::V1 { from: sender_addr, <b>to</b>, amount, asset_type, new_available_balance, auditor_hint });

        // Re-asserting dispatchable FA functionality that charges fees on withdraw/deposit was not invoked.
        <b>assert</b>!(amount == before - <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(pool_fa_store), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA">E_UNSAFE_DISPATCHABLE_FA</a>));
    } <b>else</b> {
        <a href="event.md#0x1_event_emit">event::emit</a>(Normalized::V1 { addr: sender_addr, asset_type, new_available_balance, auditor_hint });
    };
}
</code></pre>



</details>

<a id="0x1_confidential_asset_confidential_transfer_raw"></a>

## Function `confidential_transfer_raw`

Deserializes cryptographic data and forwards to <code>confidential_transfer</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_confidential_transfer_raw">confidential_transfer_raw</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, new_balance_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_R_eff_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_sender: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_recip: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_eff_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, ek_volun_auds: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, amount_R_volun_auds: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, zkrp_new_balance: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_amount: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, memo: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_confidential_transfer_raw">confidential_transfer_raw</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    new_balance_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_R_eff_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // new balance R component for the *effective* auditor only
    amount_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    amount_R_sender: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    amount_R_recip: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    amount_R_eff_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // amount R components for the *effective* auditor only
    ek_volun_auds: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // contains EKs for the *voluntary* auditors only
    amount_R_volun_auds: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, // amount R components for the *voluntary* auditors only
    zkrp_new_balance: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_amount: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    memo: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> compressed_new_balance = new_compressed_available_from_bytes(new_balance_P, new_balance_R, new_balance_R_eff_aud);

    <b>let</b> compressed_amount = <a href="confidential_amount.md#0x1_confidential_amount_new_compressed_from_bytes">confidential_amount::new_compressed_from_bytes</a>(
        amount_P, amount_R_sender, amount_R_recip, amount_R_eff_aud, amount_R_volun_auds,
    );

    <b>let</b> compressed_ek_volun_auds = ek_volun_auds.map(|bytes| {
        new_compressed_point_from_bytes(bytes).extract()
    });

    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
    <b>let</b> zkrp_amount = bulletproofs::range_proof_from_bytes(zkrp_amount);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(sigma_proto_comm, sigma_proto_resp);
    <b>let</b> proof = TransferProof::V1 {
        compressed_new_balance,
        compressed_amount,
        compressed_ek_volun_auds,
        zkrp_new_balance, zkrp_amount, sigma
    };

    <a href="confidential_asset.md#0x1_confidential_asset_confidential_transfer">confidential_transfer</a>(
        sender,
        asset_type,
        <b>to</b>,
        proof,
        memo,
    )
}
</code></pre>



</details>

<a id="0x1_confidential_asset_confidential_transfer"></a>

## Function `confidential_transfer`

Transfers a secret amount of tokens from sender's available balance to recipient's pending balance.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_confidential_transfer">confidential_transfer</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, proof: <a href="confidential_asset.md#0x1_confidential_asset_TransferProof">confidential_asset::TransferProof</a>, memo: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_confidential_transfer">confidential_transfer</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_TransferProof">TransferProof</a>,
    memo: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_safe_for_confidentiality">is_safe_for_confidentiality</a>(&asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_UNSAFE_DISPATCHABLE_FA">E_UNSAFE_DISPATCHABLE_FA</a>));
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));
    <b>assert</b>!(!<a href="confidential_asset.md#0x1_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(<b>to</b>, asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>));

    <b>let</b> from = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>assert</b>!(from != <b>to</b>, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_SELF_TRANSFER">E_SELF_TRANSFER</a>));
    <b>let</b> effective_auditor = <a href="confidential_asset.md#0x1_confidential_asset_get_effective_auditor_config">get_effective_auditor_config</a>(asset_type);
    <b>let</b> ek_sender = <a href="confidential_asset.md#0x1_confidential_asset_get_encryption_key">get_encryption_key</a>(from, asset_type);
    <b>let</b> ek_recip = <a href="confidential_asset.md#0x1_confidential_asset_get_encryption_key">get_encryption_key</a>(<b>to</b>, asset_type);
    <b>let</b> old_balance = <a href="confidential_asset.md#0x1_confidential_asset_get_available_balance">get_available_balance</a>(from, asset_type);

    // Note: Sender's amount in `TransferProof::compressed_amount::compressed_R_sender` is not used here; only included so it can be indexed for dapps that need it
    <b>let</b> (compressed_new_balance, recipient_amount, amount, ek_volun_auds) =
        <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_transfer_proof">assert_valid_transfer_proof</a>(
            sender, <b>to</b>, asset_type,
            &ek_sender, &ek_recip,
            &old_balance, &effective_auditor.config.ek,
            proof
        );

    // Update recipient's confidential store
    <b>let</b> recip_ca_store = <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(<b>to</b>, asset_type);
    <b>let</b> new_pending_balance = add_assign_pending(&<b>mut</b> recip_ca_store.pending_balance, &recipient_amount);
    recip_ca_store.transfers_received += 1;

    <b>assert</b>!(
        recip_ca_store.transfers_received &lt;= <a href="confidential_asset.md#0x1_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>)
    );

    // Update sender's confidential store
    <b>let</b> sender_ca_store = <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(from, asset_type);
    sender_ca_store.normalized = <b>true</b>;
    sender_ca_store.available_balance = compressed_new_balance;
    sender_ca_store.<a href="confidential_asset.md#0x1_confidential_asset_update_auditor_hint">update_auditor_hint</a>(&effective_auditor); // enables auditor <b>to</b> later tell whether their balance ciphertext is stale

    <a href="event.md#0x1_event_emit">event::emit</a>(Transferred::V1 {
        from, <b>to</b>, asset_type, amount, ek_volun_auds,
        sender_auditor_hint: sender_ca_store.auditor_hint,
        new_sender_available_balance: compressed_new_balance,
        new_recip_pending_balance: new_pending_balance,
        memo,
    });
}
</code></pre>



</details>

<a id="0x1_confidential_asset_rotate_encryption_key_raw"></a>

## Function `rotate_encryption_key_raw`

Deserializes cryptographic data and forwards to <code>rotate_encryption_key</code>.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rotate_encryption_key_raw">rotate_encryption_key_raw</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_ek: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, resume_incoming_transfers: bool, new_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rotate_encryption_key_raw">rotate_encryption_key_raw</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_ek: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    resume_incoming_transfers: bool,
    new_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
    sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
    sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    // Just parse stuff and forward <b>to</b> the more type-safe function
    <b>let</b> compressed_new_ek = new_compressed_point_from_bytes(new_ek).extract();
    <b>let</b> compressed_new_R = deserialize_compressed_points(new_R);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x1_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(
        sigma_proto_comm, sigma_proto_resp
    );

    <a href="confidential_asset.md#0x1_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(
        sender, asset_type,
        KeyRotationProof::V1 { compressed_new_ek, compressed_new_R, sigma },
        resume_incoming_transfers
    );
}
</code></pre>



</details>

<a id="0x1_confidential_asset_rotate_encryption_key"></a>

## Function `rotate_encryption_key`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, proof: <a href="confidential_asset.md#0x1_confidential_asset_KeyRotationProof">confidential_asset::KeyRotationProof</a>, resume_incoming_transfers: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_KeyRotationProof">KeyRotationProof</a>,
    resume_incoming_transfers: bool,
) {
    // Step 1: Assert (a) incoming transfers are paused & (b) pending balance is zero / <b>has</b> been rolled over
    <b>let</b> ca_store = <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type);
    // (a) Assert incoming transfers are paused & unpause them after, <b>if</b> flag is set.
    <b>assert</b>!(ca_store.pause_incoming, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_INCOMING_TRANSFERS_NOT_PAUSED">E_INCOMING_TRANSFERS_NOT_PAUSED</a>));
    // (b) The user must have called `rollover_pending_balance` before rotating their key.
    <b>assert</b>!(
        ca_store.pending_balance.is_zero(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>)
    );
    // Over-asserting invariants, in an abundance of caution.
    <b>assert</b>!(
        ca_store.transfers_received == 0,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>)
    );

    // Step 2: Verify the $\Sigma$-protocol proof of correct re-encryption
    <b>let</b> (compressed_new_ek, compressed_new_R) = <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_key_rotation_proof">assert_valid_key_rotation_proof</a>(
        owner, asset_type, &ca_store.ek, &ca_store.available_balance, proof
    );

    // Step 3: Install the new EK and the new re-encrypted available balance
    ca_store.ek = compressed_new_ek;
    // We're just updating the available balance's EK-dependant R component & leaving the pending balance the same.
    <a href="confidential_balance.md#0x1_confidential_balance_set_available_R">confidential_balance::set_available_R</a>(&<b>mut</b> ca_store.available_balance, compressed_new_R);
    <b>if</b> (resume_incoming_transfers) {
        ca_store.pause_incoming = <b>false</b>;
        <a href="event.md#0x1_event_emit">event::emit</a>(IncomingTransfersPauseChanged::V1 { addr: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type, paused: <b>false</b> });
    };
    <a href="event.md#0x1_event_emit">event::emit</a>(KeyRotated::V1 {
        addr: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type, new_ek: compressed_new_ek, new_available_balance: ca_store.available_balance,
    });
}
</code></pre>



</details>

<a id="0x1_confidential_asset_normalize_raw"></a>

## Function `normalize_raw`

Deserializes cryptographic data and ultimately forwards to <code>withdraw_to</code> with amount = 0.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_normalize_raw">normalize_raw</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_balance_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_R_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, zkrp_new_balance: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_normalize_raw">normalize_raw</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_balance_P: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_R: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_R_aud: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,  // effective auditor's R component
    zkrp_new_balance: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> user = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>assert</b>!(!<a href="confidential_asset.md#0x1_confidential_asset_is_normalized">is_normalized</a>(user, asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_ALREADY_NORMALIZED">E_ALREADY_NORMALIZED</a>));

    <a href="confidential_asset.md#0x1_confidential_asset_withdraw_to_raw">withdraw_to_raw</a>(
        sender, asset_type, user, 0,
        new_balance_P, new_balance_R, new_balance_R_aud,
        zkrp_new_balance, sigma_proto_comm, sigma_proto_resp
    );
}
</code></pre>



</details>

<a id="0x1_confidential_asset_normalize"></a>

## Function `normalize`

Re-encrypts the available balance to ensure all chunks are within 16-bit bounds (required before rollover).


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_normalize">normalize</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, proof: <a href="confidential_asset.md#0x1_confidential_asset_WithdrawalProof">confidential_asset::WithdrawalProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_normalize">normalize</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_WithdrawalProof">WithdrawalProof</a>
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> user = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>assert</b>!(!<a href="confidential_asset.md#0x1_confidential_asset_is_normalized">is_normalized</a>(user, asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_ALREADY_NORMALIZED">E_ALREADY_NORMALIZED</a>));

    // Normalization is withdrawal <b>with</b> v = 0
    <a href="confidential_asset.md#0x1_confidential_asset_withdraw_to">withdraw_to</a>(sender, asset_type, user, 0, proof);
}
</code></pre>



</details>

<a id="0x1_confidential_asset_rollover_pending_balance"></a>

## Function `rollover_pending_balance`

Rolls over pending balance into available balance, resetting pending to zero.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rollover_pending_balance">rollover_pending_balance</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rollover_pending_balance">rollover_pending_balance</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> user = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>let</b> ca_store = <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(user, asset_type);

    <b>assert</b>!(ca_store.normalized, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_NORMALIZATION_REQUIRED">E_NORMALIZATION_REQUIRED</a>));
    <b>assert</b>!(ca_store.transfers_received &gt; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_NOTHING_TO_ROLLOVER">E_NOTHING_TO_ROLLOVER</a>));

    ca_store.available_balance.add_assign_available_excluding_auditor(&ca_store.pending_balance);
    // Note: R_aud components [must] remain stale, but will be refreshed on the next normalize/withdraw/transfer
    // Note: Since this function does not <b>update</b> the *auditor's* available balance, we do not <b>update</b> the auditor hint.

    ca_store.normalized = <b>false</b>;
    ca_store.transfers_received = 0;
    ca_store.pending_balance = new_zero_pending_compressed();

    <a href="event.md#0x1_event_emit">event::emit</a>(RolledOver::V1 { addr: user, asset_type, new_available_balance: ca_store.available_balance });
}
</code></pre>



</details>

<a id="0x1_confidential_asset_rollover_pending_balance_and_pause"></a>

## Function `rollover_pending_balance_and_pause`

Rollover + pause incoming transfers (required before key rotation).


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rollover_pending_balance_and_pause">rollover_pending_balance_and_pause</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_rollover_pending_balance_and_pause">rollover_pending_balance_and_pause</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_rollover_pending_balance">rollover_pending_balance</a>(sender, asset_type);
    <a href="confidential_asset.md#0x1_confidential_asset_set_incoming_transfers_paused">set_incoming_transfers_paused</a>(sender, asset_type, <b>true</b>);
}
</code></pre>



</details>

<a id="0x1_confidential_asset_set_incoming_transfers_paused"></a>

## Function `set_incoming_transfers_paused`

Pauses or resumes incoming transfers. Pausing is required before key rotation.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_incoming_transfers_paused">set_incoming_transfers_paused</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, paused: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_incoming_transfers_paused">set_incoming_transfers_paused</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    paused: bool
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> ca_store = <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type);
    <b>let</b> old_paused = ca_store.pause_incoming;
    <b>if</b> (old_paused != paused) {
        ca_store.pause_incoming = paused;
        <a href="event.md#0x1_event_emit">event::emit</a>(IncomingTransfersPauseChanged::V1 { addr: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type, paused });
    }
}
</code></pre>



</details>

<a id="0x1_confidential_asset_set_allow_listing"></a>

## Function `set_allow_listing`

Enables or disables the allow list for confidential transfers.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_allow_listing">set_allow_listing</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enabled: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_allow_listing">set_allow_listing</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enabled: bool) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> global_config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework);
    <b>let</b> old_allow_listing_enabled = global_config.allow_list_enabled;
    <b>if</b> (old_allow_listing_enabled != enabled) {
        global_config.allow_list_enabled = enabled;
        <a href="event.md#0x1_event_emit">event::emit</a>(AllowListingChanged::V1 { enabled });
    }
}
</code></pre>



</details>

<a id="0x1_confidential_asset_set_confidentiality_for_apt"></a>

## Function `set_confidentiality_for_apt`

Enables or disables confidentiality for the APT token.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_confidentiality_for_apt">set_confidentiality_for_apt</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allowed: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_confidentiality_for_apt">set_confidentiality_for_apt</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, allowed: bool) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> asset_type = <a href="object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;(@aptos_fungible_asset);
    <a href="confidential_asset.md#0x1_confidential_asset_set_confidentiality_for_asset_type">set_confidentiality_for_asset_type</a>(aptos_framework, asset_type, allowed)
}
</code></pre>



</details>

<a id="0x1_confidential_asset_set_confidentiality_for_asset_type"></a>

## Function `set_confidentiality_for_asset_type`

When allow listing is on, this enables or disables confidential transfers for a specific asset type. In contrast,
if allow listing is disabled, this aborts. Note that, in this case, <code>is_confidentiality_enabled_for_asset_type</code>
will correctly return <code><b>false</b></code> for any asset type.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_confidentiality_for_asset_type">set_confidentiality_for_asset_type</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, allowed: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_confidentiality_for_asset_type">set_confidentiality_for_asset_type</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    allowed: bool
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    // When allow listing is disabled, updates <b>to</b> `AssetConfig::V1::allowed` are not meaningful, so we forbid them.
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_is_allow_listing_required">is_allow_listing_required</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_ALLOW_LISTING_IS_DISABLED">E_ALLOW_LISTING_IS_DISABLED</a>));

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(<a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type));
    <b>if</b> (config.allowed != allowed) {
        config.allowed = allowed;
        <a href="event.md#0x1_event_emit">event::emit</a>(ConfidentialityForAssetTypeChanged::V1 { asset_type, allowed });
    }
}
</code></pre>



</details>

<a id="0x1_confidential_asset_set_asset_specific_auditor"></a>

## Function `set_asset_specific_auditor`

Sets or removes the auditor for a specific asset type. Epoch increments only on install/change.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_asset_specific_auditor">set_asset_specific_auditor</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, auditor_ek: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_asset_specific_auditor">set_asset_specific_auditor</a>(
    aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    auditor_ek: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> config_addr = <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type);
    <b>if</b> (<a href="confidential_asset.md#0x1_confidential_asset_update_auditor">update_auditor</a>(&<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr).auditor, auditor_ek)) {
        <b>let</b> new = <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr).auditor;
        <a href="event.md#0x1_event_emit">event::emit</a>(AssetSpecificAuditorChanged::V1 { asset_type, new });
    }
}
</code></pre>



</details>

<a id="0x1_confidential_asset_set_global_auditor"></a>

## Function `set_global_auditor`

Sets or removes the global auditor (fallback when no asset-specific auditor). Epoch increments only on install/change.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_global_auditor">set_global_auditor</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, auditor_ek: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_set_global_auditor">set_global_auditor</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, auditor_ek: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="confidential_asset.md#0x1_confidential_asset_update_auditor">update_auditor</a>(&<b>mut</b> config.global_auditor, auditor_ek)) {
        <a href="event.md#0x1_event_emit">event::emit</a>(GlobalAuditorChanged::V1 { new: config.global_auditor });
    }
}
</code></pre>



</details>

<a id="0x1_confidential_asset_update_auditor"></a>

## Function `update_auditor`

Shared logic for installing/changing/removing an auditor EK. Validates non-identity, increments epoch on install/change.
Returns <code><b>true</b></code> if the auditor config actually changed.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_update_auditor">update_auditor</a>(auditor: &<b>mut</b> <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a>, new_ek_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_update_auditor">update_auditor</a>(auditor: &<b>mut</b> <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">AuditorConfig</a>, new_ek_bytes: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool {
    <b>let</b> new_ek = new_ek_bytes.map(|ek| new_compressed_point_from_bytes(ek).extract());

    <b>if</b> (new_ek.is_some()) {
        <b>assert</b>!(!new_ek.borrow().is_identity(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_AUDITOR_EK_IS_IDENTITY">E_AUDITOR_EK_IS_IDENTITY</a>));
    };

    // Increment epoch only when installing or changing the EK (not when removing):
    // i.e.,  should_increment = [ Is None --&gt; Some(ek) ? ] or [ Is Some(<b>old</b>) --&gt; Some(new), <b>with</b> new != <b>old</b> ? ]
    <b>let</b> should_increment = <b>if</b> (new_ek.is_some()) {
        <b>if</b> (auditor.ek.is_some()) {
            !new_ek.borrow().compressed_point_equals(auditor.ek.borrow())   // i.e., new != <b>old</b>
        } <b>else</b> {
            <b>true</b> // None --&gt; Some(ek): installing
        }
    } <b>else</b> {
        <b>false</b> // removing or no-op
    };

    // Changed <b>if</b>: epoch incremented (install/change), or EK removed (Some → None)
    <b>let</b> is_removal = auditor.ek.is_some() && new_ek.is_none();
    <b>let</b> changed = should_increment || is_removal;

    auditor.epoch += <b>if</b> (should_increment) { 1 } <b>else</b> { 0 };
    auditor.ek = new_ek;

    changed
}
</code></pre>



</details>

<a id="0x1_confidential_asset_has_confidential_store"></a>

## Function `has_confidential_store`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_has_confidential_store">has_confidential_store</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_has_confidential_store">has_confidential_store</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): bool {
    <b>exists</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type))
}
</code></pre>



</details>

<a id="0x1_confidential_asset_is_confidentiality_enabled_for_asset_type"></a>

## Function `is_confidentiality_enabled_for_asset_type`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>if</b> (!<a href="confidential_asset.md#0x1_confidential_asset_is_allow_listing_required">is_allow_listing_required</a>()) {
        <b>return</b> <b>true</b>
    };

    <b>let</b> asset_config_address = <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);
    <b>if</b> (!<b>exists</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address)) {
        <b>return</b> <b>false</b>
    };

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address).allowed
}
</code></pre>



</details>

<a id="0x1_confidential_asset_is_allow_listing_required"></a>

## Function `is_allow_listing_required`

If the allow list is enabled, only asset types from the allow list can be transferred confidentially. Otherwise, all asset types are allowed.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_allow_listing_required">is_allow_listing_required</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_allow_listing_required">is_allow_listing_required</a>(): bool <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework).allow_list_enabled
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_pending_balance"></a>

## Function `get_pending_balance`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_pending_balance">get_pending_balance</a>(owner: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_pending_balance">get_pending_balance</a>(
    owner: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): CompressedBalance&lt;Pending&gt; <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(owner, asset_type).pending_balance
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_available_balance"></a>

## Function `get_available_balance`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_available_balance">get_available_balance</a>(owner: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_available_balance">get_available_balance</a>(
    owner: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): CompressedBalance&lt;Available&gt; <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(owner, asset_type).available_balance
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_encryption_key"></a>

## Function `get_encryption_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_encryption_key">get_encryption_key</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_encryption_key">get_encryption_key</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): CompressedRistretto <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).ek
}
</code></pre>



</details>

<a id="0x1_confidential_asset_is_normalized"></a>

## Function `is_normalized`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_normalized">is_normalized</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_is_normalized">is_normalized</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): bool <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).normalized
}
</code></pre>



</details>

<a id="0x1_confidential_asset_incoming_transfers_paused"></a>

## Function `incoming_transfers_paused`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).pause_incoming
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_effective_auditor_hint"></a>

## Function `get_effective_auditor_hint`

Returns the auditor hint for a user's confidential store, indicating which auditor the balance ciphertext is encrypted for.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_effective_auditor_hint">get_effective_auditor_hint</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorHint">confidential_asset::EffectiveAuditorHint</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_effective_auditor_hint">get_effective_auditor_hint</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): Option&lt;<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorHint">EffectiveAuditorHint</a>&gt; <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).auditor_hint
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_asset_specific_auditor_config"></a>

## Function `get_asset_specific_auditor_config`

Note: Dapp developers should **not** need to call this function, which is why (for now) it is marked private.

This ignores the global auditor, if any, and only returns the asset-specific auditor config. Furthermore, it returns
the auditor config even if the asset_type is no longer allow-listed.


<pre><code>#[view]
<b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_specific_auditor_config">get_asset_specific_auditor_config</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_specific_auditor_config">get_asset_specific_auditor_config</a>(
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">AuditorConfig</a> <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> asset_config_address = <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address).auditor
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_global_auditor_config"></a>

## Function `get_global_auditor_config`

Note: Dapp developers should **not** need to call this function, which is why (for now) it is marked private.

This ignores asset-specific auditors, if any, and only returns the global auditor config. Furthermore, it returns
the auditor config even if the asset_type is no longer allow-listed.


<pre><code>#[view]
<b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_global_auditor_config">get_global_auditor_config</a>(): <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">confidential_asset::AuditorConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_global_auditor_config">get_global_auditor_config</a>(): <a href="confidential_asset.md#0x1_confidential_asset_AuditorConfig">AuditorConfig</a> <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework).global_auditor
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_effective_auditor_config"></a>

## Function `get_effective_auditor_config`

Returns the effective auditor: asset-specific if set, else global.
Used by dapp developers to fetch the right auditor EK to create withdraw, normalize or transfer transactions.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_effective_auditor_config">get_effective_auditor_config</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorConfig">confidential_asset::EffectiveAuditorConfig</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_effective_auditor_config">get_effective_auditor_config</a>(
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): <a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorConfig">EffectiveAuditorConfig</a> <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> config_addr = <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type); // first, check asset-specific auditor
    <b>if</b> (<b>exists</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr)) {
        <b>return</b> EffectiveAuditorConfig::V1 {
            is_global: <b>false</b>,
            config: <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr).auditor
        };
    };

    EffectiveAuditorConfig::V1 {      // otherwise, fall back <b>to</b> <b>global</b> auditor
        is_global: <b>true</b>,
        config: <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;( @aptos_framework).global_auditor
    }
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_total_confidential_supply"></a>

## Function `get_total_confidential_supply`

Returns the circulating supply of the confidential asset.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_total_confidential_supply">get_total_confidential_supply</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_total_confidential_supply">get_total_confidential_supply</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64 <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="fungible_asset.md#0x1_fungible_asset_balance">fungible_asset::balance</a>(<a href="confidential_asset.md#0x1_confidential_asset_get_pool_fa_store">get_pool_fa_store</a>(asset_type))
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_num_transfers_received"></a>

## Function `get_num_transfers_received`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_num_transfers_received">get_num_transfers_received</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_num_transfers_received">get_num_transfers_received</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): u64 <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).transfers_received
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_max_transfers_before_rollover"></a>

## Function `get_max_transfers_before_rollover`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_max_transfers_before_rollover">get_max_transfers_before_rollover</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_max_transfers_before_rollover">get_max_transfers_before_rollover</a>(): u64 {
    <a href="confidential_asset.md#0x1_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>
}
</code></pre>



</details>

<a id="0x1_confidential_asset_update_auditor_hint"></a>

## Function `update_auditor_hint`

Updates the auditor hint stored in a confidential store to track the currently-set on-chain effective auditor


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_update_auditor_hint">update_auditor_hint</a>(self: &<b>mut</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">confidential_asset::ConfidentialStore</a>, effective_auditor: &<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorConfig">confidential_asset::EffectiveAuditorConfig</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_update_auditor_hint">update_auditor_hint</a>(self: &<b>mut</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>, effective_auditor: &<a href="confidential_asset.md#0x1_confidential_asset_EffectiveAuditorConfig">EffectiveAuditorConfig</a>) {
    <b>if</b> (effective_auditor.config.ek.is_none()) {
        // If there is no effective auditor EK set, we unset the effective auditor hint
        self.auditor_hint = std::option::none()
    } <b>else</b> {
        // Otherwise, we <b>update</b> the effective auditor hint: type [<b>global</b> or asset-specific] and epoch
        self.auditor_hint = std::option::some(EffectiveAuditorHint::V1 {
            is_global: effective_auditor.is_global,
            epoch: effective_auditor.config.epoch,
        })
    };
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_asset_config_address"></a>

## Function `get_asset_config_address`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> config_ext = &<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework).extend_ref;
    <b>let</b> config_ext_address = config_ext.address_from_extend_ref();
    <a href="object.md#0x1_object_create_object_address">object::create_object_address</a>(&config_ext_address, <a href="confidential_asset.md#0x1_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type))
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_asset_config_address_or_create"></a>

## Function `get_asset_config_address_or_create`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> addr = <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);

    <b>if</b> (!<b>exists</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>&gt;(addr)) {
        <b>let</b> asset_config_signer = <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_signer">get_asset_config_signer</a>(asset_type);

        <b>move_to</b>(
            &asset_config_signer,
            // We disallow the asset type from being made confidential since this function is called in a lot of different contexts.
            AssetConfig::V1 { allowed: <b>false</b>, auditor: AuditorConfig::V1 { ek: std::option::none(), epoch: 0 } }
        );
    };

    addr
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_global_config_signer"></a>

## Function `get_global_config_signer`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_global_config_signer">get_global_config_signer</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_global_config_signer">get_global_config_signer</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework).extend_ref.generate_signer_for_extending()
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_pool_fa_store"></a>

## Function `get_pool_fa_store`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_pool_fa_store">get_pool_fa_store</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_pool_fa_store">get_pool_fa_store</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): Object&lt;FungibleStore&gt; {
    <b>let</b> global_config_addr = <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework).extend_ref.address_from_extend_ref();
    <b>assert</b>!(<a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_fungible_store::primary_store_exists</a>(global_config_addr, asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE">E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE</a>));
    <a href="primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_fungible_store::primary_store</a>(global_config_addr, asset_type)
}
</code></pre>



</details>

<a id="0x1_confidential_asset_ensure_pool_fa_store"></a>

## Function `ensure_pool_fa_store`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_ensure_pool_fa_store">ensure_pool_fa_store</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_FungibleStore">fungible_asset::FungibleStore</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_ensure_pool_fa_store">ensure_pool_fa_store</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): Object&lt;FungibleStore&gt; {
    <b>let</b> global_config_addr = <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework).extend_ref.address_from_extend_ref();
    <a href="primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(global_config_addr, asset_type)
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_confidential_store_signer"></a>

## Function `get_confidential_store_signer`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_signer">get_confidential_store_signer</a>(user: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_signer">get_confidential_store_signer</a>(user: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="object.md#0x1_object_create_named_object">object::create_named_object</a>(user, <a href="confidential_asset.md#0x1_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type)).generate_signer()
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_confidential_store_address"></a>

## Function `get_confidential_store_address`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> {
    <a href="object.md#0x1_object_create_object_address">object::create_object_address</a>(&user, <a href="confidential_asset.md#0x1_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type))
}
</code></pre>



</details>

<a id="0x1_confidential_asset_borrow_confidential_store"></a>

## Function `borrow_confidential_store`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">confidential_asset::ConfidentialStore</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>));
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type))
}
</code></pre>



</details>

<a id="0x1_confidential_asset_borrow_confidential_store_mut"></a>

## Function `borrow_confidential_store_mut`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(user: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<b>mut</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">confidential_asset::ConfidentialStore</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(user: <b>address</b>, asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<b>mut</b> <a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x1_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x1_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>));
    <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x1_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type))
}
</code></pre>



</details>

<a id="0x1_confidential_asset_get_asset_config_signer"></a>

## Function `get_asset_config_signer`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_signer">get_asset_config_signer</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_get_asset_config_signer">get_asset_config_signer</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> config_ext = &<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x1_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_framework).extend_ref;
    <b>let</b> config_ext_signer = config_ext.generate_signer_for_extending();

    <b>let</b> config_ctor =
        &<a href="object.md#0x1_object_create_named_object">object::create_named_object</a>(&config_ext_signer, <a href="confidential_asset.md#0x1_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type));

    config_ctor.generate_signer()
}
</code></pre>



</details>

<a id="0x1_confidential_asset_construct_confidential_store_seed"></a>

## Function `construct_confidential_store_seed`

Unique seed per (user, asset-type) for the ConfidentialStore object address.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(
        &<a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_format2">string_utils::format2</a>(
            &b"<a href="confidential_asset.md#0x1_confidential_asset">confidential_asset</a>::{}::asset_type::{}::<a href="confidential_asset.md#0x1_confidential_asset_ConfidentialStore">ConfidentialStore</a>",
            @aptos_framework,
            asset_type.object_address()
        )
    )
}
</code></pre>



</details>

<a id="0x1_confidential_asset_construct_asset_config_seed"></a>

## Function `construct_asset_config_seed`

Unique seed per asset-type for the AssetConfig object address.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(
        &<a href="../../aptos-stdlib/doc/string_utils.md#0x1_string_utils_format2">string_utils::format2</a>(
            &b"<a href="confidential_asset.md#0x1_confidential_asset">confidential_asset</a>::{}::asset_type::{}::<a href="confidential_asset.md#0x1_confidential_asset_AssetConfig">AssetConfig</a>",
            @aptos_framework,
            asset_type.object_address()
        )
    )
}
</code></pre>



</details>

<a id="0x1_confidential_asset_assert_valid_registration_proof"></a>

## Function `assert_valid_registration_proof`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_registration_proof">assert_valid_registration_proof</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, proof: <a href="confidential_asset.md#0x1_confidential_asset_RegistrationProof">confidential_asset::RegistrationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_registration_proof">assert_valid_registration_proof</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: &CompressedRistretto,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_RegistrationProof">RegistrationProof</a>
) {
    <b>let</b> RegistrationProof::V1 { sigma } = proof;
    <b>let</b> stmt = <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_new_registration_statement">sigma_protocol_registration::new_registration_statement</a>(*ek);
    <b>let</b> session = <a href="sigma_protocol_registration.md#0x1_sigma_protocol_registration_new_session">sigma_protocol_registration::new_session</a>(sender, asset_type);
    session.assert_verifies(&stmt, &sigma);
}
</code></pre>



</details>

<a id="0x1_confidential_asset_assert_valid_withdrawal_proof"></a>

## Function `assert_valid_withdrawal_proof`

Verifies range proof + $\Sigma$-protocol for withdrawal. Returns compressed new balance.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_withdrawal_proof">assert_valid_withdrawal_proof</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, amount: u64, old_balance: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, compressed_ek_aud: &<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, proof: <a href="confidential_asset.md#0x1_confidential_asset_WithdrawalProof">confidential_asset::WithdrawalProof</a>): <a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_withdrawal_proof">assert_valid_withdrawal_proof</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: &CompressedRistretto,
    amount: u64,
    old_balance: &CompressedBalance&lt;Available&gt;,
    compressed_ek_aud: &Option&lt;CompressedRistretto&gt;,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_WithdrawalProof">WithdrawalProof</a>
): CompressedBalance&lt;Available&gt; {
    <b>let</b> WithdrawalProof::V1 { compressed_new_balance, zkrp_new_balance, sigma } = proof;

    <b>let</b> v = new_scalar_from_u64(amount);

    <b>let</b> (stmt, new_balance_P) = <a href="sigma_protocol_withdraw.md#0x1_sigma_protocol_withdraw_new_withdrawal_statement">sigma_protocol_withdraw::new_withdrawal_statement</a>(
        *ek, old_balance, &compressed_new_balance, compressed_ek_aud, v,
    );
    <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_assert_valid_range_proof">confidential_range_proofs::assert_valid_range_proof</a>(&new_balance_P, &zkrp_new_balance);

    <b>let</b> session = <a href="sigma_protocol_withdraw.md#0x1_sigma_protocol_withdraw_new_session">sigma_protocol_withdraw::new_session</a>(sender, asset_type, compressed_ek_aud.is_some());
    session.assert_verifies(&stmt, &sigma);
    compressed_new_balance
}
</code></pre>



</details>

<a id="0x1_confidential_asset_assert_valid_transfer_proof"></a>

## Function `assert_valid_transfer_proof`

Verifies range proofs + $\Sigma$-protocol for transfer. Returns (new_balance, recipient_pending).


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_transfer_proof">assert_valid_transfer_proof</a>(sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient_addr: <b>address</b>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, compressed_ek_sender: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, compressed_ek_recip: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, compressed_old_balance: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, compressed_ek_eff_aud: &<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, proof: <a href="confidential_asset.md#0x1_confidential_asset_TransferProof">confidential_asset::TransferProof</a>): (<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, <a href="confidential_balance.md#0x1_confidential_balance_Balance">confidential_balance::Balance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Pending">confidential_balance::Pending</a>&gt;, <a href="confidential_amount.md#0x1_confidential_amount_CompressedAmount">confidential_amount::CompressedAmount</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_transfer_proof">assert_valid_transfer_proof</a>(
    sender: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient_addr: <b>address</b>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    compressed_ek_sender: &CompressedRistretto,
    compressed_ek_recip: &CompressedRistretto,
    compressed_old_balance: &CompressedBalance&lt;Available&gt;,
    compressed_ek_eff_aud: &Option&lt;CompressedRistretto&gt;,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_TransferProof">TransferProof</a>
): (
    CompressedBalance&lt;Available&gt;,
    Balance&lt;Pending&gt;,
    CompressedAmount,
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
) {

    <b>let</b> TransferProof::V1 {
        compressed_new_balance, compressed_amount,
        compressed_ek_volun_auds,
        zkrp_new_balance, zkrp_amount, sigma
    } = proof;

    <b>let</b> has_effective_auditor = compressed_ek_eff_aud.is_some();
    <b>let</b> num_volun_auditors = compressed_ek_volun_auds.length();

    // Auditor count checks are performed inside new_transfer_statement
    <b>let</b> (stmt, new_balance_P, recip_pending) = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_new_transfer_statement">sigma_protocol_transfer::new_transfer_statement</a>(
        *compressed_ek_sender, *compressed_ek_recip,
        compressed_old_balance, &compressed_new_balance,
        &compressed_amount,
        compressed_ek_eff_aud, &compressed_ek_volun_auds,
    );

    <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_assert_valid_range_proof">confidential_range_proofs::assert_valid_range_proof</a>(recip_pending.get_P(), &zkrp_amount);
    <a href="confidential_range_proofs.md#0x1_confidential_range_proofs_assert_valid_range_proof">confidential_range_proofs::assert_valid_range_proof</a>(&new_balance_P, &zkrp_new_balance);

    <b>let</b> session = <a href="sigma_protocol_transfer.md#0x1_sigma_protocol_transfer_new_session">sigma_protocol_transfer::new_session</a>(
        sender, recipient_addr, asset_type, has_effective_auditor, num_volun_auditors,
    );
    session.assert_verifies(&stmt, &sigma);

    (compressed_new_balance, recip_pending, compressed_amount, compressed_ek_volun_auds)
}
</code></pre>



</details>

<a id="0x1_confidential_asset_assert_valid_key_rotation_proof"></a>

## Function `assert_valid_key_rotation_proof`



<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_key_rotation_proof">assert_valid_key_rotation_proof</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, old_ek: &<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, old_balance: &<a href="confidential_balance.md#0x1_confidential_balance_CompressedBalance">confidential_balance::CompressedBalance</a>&lt;<a href="confidential_balance.md#0x1_confidential_balance_Available">confidential_balance::Available</a>&gt;, proof: <a href="confidential_asset.md#0x1_confidential_asset_KeyRotationProof">confidential_asset::KeyRotationProof</a>): (<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x1_confidential_asset_assert_valid_key_rotation_proof">assert_valid_key_rotation_proof</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    old_ek: &CompressedRistretto,
    old_balance: &CompressedBalance&lt;Available&gt;,
    proof: <a href="confidential_asset.md#0x1_confidential_asset_KeyRotationProof">KeyRotationProof</a>
): (CompressedRistretto, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;) {
    <b>let</b> KeyRotationProof::V1 { compressed_new_ek, compressed_new_R, sigma } = proof;

    <b>let</b> stmt = <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_new_key_rotation_statement">sigma_protocol_key_rotation::new_key_rotation_statement</a>(
        *old_ek,
        compressed_new_ek,
        old_balance.get_compressed_R(),
        &compressed_new_R,
    );

    <b>let</b> session = <a href="sigma_protocol_key_rotation.md#0x1_sigma_protocol_key_rotation_new_session">sigma_protocol_key_rotation::new_session</a>(owner, asset_type);
    session.assert_verifies(&stmt, &sigma);

    (compressed_new_ek, compressed_new_R)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
