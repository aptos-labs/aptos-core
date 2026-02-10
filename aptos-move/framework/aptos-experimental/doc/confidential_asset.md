
<a id="0x7_confidential_asset"></a>

# Module `0x7::confidential_asset`

This module implements the Confidential Asset (CA) Standard, a privacy-focused protocol for managing fungible assets (FA).
It enables private transfers by obfuscating transaction amounts while keeping sender and recipient addresses visible.


-  [Resource `ConfidentialStore`](#0x7_confidential_asset_ConfidentialStore)
-  [Resource `FAController`](#0x7_confidential_asset_FAController)
-  [Resource `FAConfig`](#0x7_confidential_asset_FAConfig)
-  [Struct `Deposited`](#0x7_confidential_asset_Deposited)
-  [Struct `Withdrawn`](#0x7_confidential_asset_Withdrawn)
-  [Struct `Transferred`](#0x7_confidential_asset_Transferred)
-  [Enum `KeyRotationProof`](#0x7_confidential_asset_KeyRotationProof)
-  [Constants](#@Constants_0)
    -  [[test_only] The confidential asset module initialization failed.](#@[test_only]_The_confidential_asset_module_initialization_failed._1)
-  [Function `init_module`](#0x7_confidential_asset_init_module)
-  [Function `init_module_for_devnet`](#0x7_confidential_asset_init_module_for_devnet)
-  [Function `register`](#0x7_confidential_asset_register)
-  [Function `deposit_to`](#0x7_confidential_asset_deposit_to)
-  [Function `deposit`](#0x7_confidential_asset_deposit)
-  [Function `withdraw_to`](#0x7_confidential_asset_withdraw_to)
-  [Function `withdraw`](#0x7_confidential_asset_withdraw)
-  [Function `confidential_transfer`](#0x7_confidential_asset_confidential_transfer)
-  [Function `rotate_encryption_key`](#0x7_confidential_asset_rotate_encryption_key)
-  [Function `rotate_encryption_key_internal`](#0x7_confidential_asset_rotate_encryption_key_internal)
-  [Function `normalize`](#0x7_confidential_asset_normalize)
-  [Function `pause_incoming_transactions`](#0x7_confidential_asset_pause_incoming_transactions)
-  [Function `resume_incoming_transactions`](#0x7_confidential_asset_resume_incoming_transactions)
-  [Function `rollover_pending_balance`](#0x7_confidential_asset_rollover_pending_balance)
-  [Function `rollover_pending_balance_and_freeze`](#0x7_confidential_asset_rollover_pending_balance_and_freeze)
-  [Function `enable_allow_listing`](#0x7_confidential_asset_enable_allow_listing)
-  [Function `disable_allow_listing`](#0x7_confidential_asset_disable_allow_listing)
-  [Function `enable_confidentiality_for_asset_type`](#0x7_confidential_asset_enable_confidentiality_for_asset_type)
-  [Function `disable_confidentiality_for_asset_type`](#0x7_confidential_asset_disable_confidentiality_for_asset_type)
-  [Function `set_auditor_for_asset_type`](#0x7_confidential_asset_set_auditor_for_asset_type)
-  [Function `set_auditor_globally`](#0x7_confidential_asset_set_auditor_globally)
-  [Function `has_confidential_store`](#0x7_confidential_asset_has_confidential_store)
-  [Function `is_confidentiality_enabled_for_asset_type`](#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type)
-  [Function `is_allow_listing_enabled`](#0x7_confidential_asset_is_allow_listing_enabled)
-  [Function `get_pending_balance`](#0x7_confidential_asset_get_pending_balance)
-  [Function `get_available_balance`](#0x7_confidential_asset_get_available_balance)
-  [Function `get_encryption_key`](#0x7_confidential_asset_get_encryption_key)
-  [Function `is_normalized`](#0x7_confidential_asset_is_normalized)
-  [Function `incoming_transfers_paused`](#0x7_confidential_asset_incoming_transfers_paused)
-  [Function `get_auditor_for_asset_type`](#0x7_confidential_asset_get_auditor_for_asset_type)
-  [Function `get_total_supply`](#0x7_confidential_asset_get_total_supply)
-  [Function `get_num_transfers_received`](#0x7_confidential_asset_get_num_transfers_received)
-  [Function `register_internal`](#0x7_confidential_asset_register_internal)
-  [Function `deposit_to_internal`](#0x7_confidential_asset_deposit_to_internal)
-  [Function `withdraw_to_internal`](#0x7_confidential_asset_withdraw_to_internal)
-  [Function `confidential_transfer_internal`](#0x7_confidential_asset_confidential_transfer_internal)
-  [Function `normalize_internal`](#0x7_confidential_asset_normalize_internal)
-  [Function `rollover_pending_balance_internal`](#0x7_confidential_asset_rollover_pending_balance_internal)
-  [Function `pause_incoming_transactions_internal`](#0x7_confidential_asset_pause_incoming_transactions_internal)
-  [Function `resume_incoming_transactions_internal`](#0x7_confidential_asset_resume_incoming_transactions_internal)
-  [Function `get_fa_config_address`](#0x7_confidential_asset_get_fa_config_address)
-  [Function `get_fa_config_address_or_create`](#0x7_confidential_asset_get_fa_config_address_or_create)
-  [Function `get_fa_controller_signer`](#0x7_confidential_asset_get_fa_controller_signer)
-  [Function `get_fa_controller_address`](#0x7_confidential_asset_get_fa_controller_address)
-  [Function `get_confidential_store_signer`](#0x7_confidential_asset_get_confidential_store_signer)
-  [Function `get_confidential_store_address`](#0x7_confidential_asset_get_confidential_store_address)
-  [Function `get_fa_config_signer`](#0x7_confidential_asset_get_fa_config_signer)
-  [Function `construct_confidential_store_seed`](#0x7_confidential_asset_construct_confidential_store_seed)
-  [Function `construct_fa_config_seed`](#0x7_confidential_asset_construct_fa_config_seed)
-  [Function `validate_auditors`](#0x7_confidential_asset_validate_auditors)
-  [Function `deserialize_auditor_eks`](#0x7_confidential_asset_deserialize_auditor_eks)
-  [Function `deserialize_auditor_amounts`](#0x7_confidential_asset_deserialize_auditor_amounts)


<pre><code><b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-framework/doc/dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset">0x1::dispatchable_fungible_asset</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset">0x1::fungible_asset</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store">0x1::primary_fungible_store</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255">0x1::ristretto255</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="confidential_balance.md#0x7_confidential_balance">0x7::confidential_balance</a>;
<b>use</b> <a href="confidential_proof.md#0x7_confidential_proof">0x7::confidential_proof</a>;
<b>use</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation">0x7::sigma_protocol_key_rotation</a>;
<b>use</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof">0x7::sigma_protocol_proof</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement">0x7::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils">0x7::sigma_protocol_utils</a>;
</code></pre>



<a id="0x7_confidential_asset_ConfidentialStore"></a>

## Resource `ConfidentialStore`

An object that stores the encrypted balances for a specific confidential asset type and owning user.
This should be thought of as a confidential variant of <code>aptos_framework::fungible_asset::FungibleStore</code>.

e.g., for Alice's confidential APT, such an object will be created and stored at an Alice-specific and APT-specific
address. It will track Alice's confidential APT balance.


<pre><code><b>struct</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pause_incoming_transfers: bool</code>
</dt>
<dd>
 Indicates if incoming transfers are paused for this asset type, which is necessary to ensure the pending
 balance does not change during a key rotation, which would invalidate that key rotation and leave the account
 in an inconsistent state.
</dd>
<dt>
<code>normalized: bool</code>
</dt>
<dd>
 A flag indicating whether the available balance is normalized. A normalized balance
 ensures that all chunks fit within the defined 16-bit bounds. This ensures that, after, roll-over all chunks
 remain 32-bit.
</dd>
<dt>
<code>transfers_received: u64</code>
</dt>
<dd>
 The number of payments received so far, which gives an upper bound on the size of the pending balance chunks
 and thus on the size of the available balance chunks, post roll-over.
</dd>
<dt>
<code>pending_balance: <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a></code>
</dt>
<dd>
 Stores the user's pending balance, which is used for accepting incoming transfers.
 Represented as four 16-bit chunks $(p_0 + 2^{16} \cdot p_1 + ... + (2^{16})^15 \cdot p_15)$ that can grow
 up to 32 bits. All payments are accepted into this pending balance, which users should roll over into their
 periodically as they run out of available balance (see <code>available_balance</code> field below).

 This separation helps protect against front-running attacks, where small incoming transfers could force
 frequent regeneration of ZK proofs.
</dd>
<dt>
<code>available_balance: <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a></code>
</dt>
<dd>
 Represents the user's balance that is available for sending payments.
 It consists of eight 16-bit chunks $(a_0 + 2^{16} \cdot a_1 + ... + (2^{16})^15 \cdot a_15)$, supporting a
 128-bit balance.
</dd>
<dt>
<code>ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>
 The encryption key associated with the user's confidential asset account, different for each asset type.
</dd>
</dl>


</details>

<a id="0x7_confidential_asset_FAController"></a>

## Resource `FAController`

A resource that represents the controller for the primary FA stores and <code><a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a></code> objects, "installed" during
<code>init_module</code> at @aptos_experimental.
TODO(upgradeability): Should we make this into an enum to make it easier to upgrade it?


<pre><code><b>struct</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> <b>has</b> key
</code></pre>



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
<code>extend_ref: <a href="../../aptos-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>
 Used to derive a signer that owns all the FAs' primary stores and <code><a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a></code> objects.
</dd>
</dl>


</details>

<a id="0x7_confidential_asset_FAConfig"></a>

## Resource `FAConfig`

An object that represents the configuration of an asset type.

TODO(upgradeability): Should we make this into an enum to make it easier to upgrade it?


<pre><code><b>struct</b> <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allowed: bool</code>
</dt>
<dd>
 Indicates whether the asset type is allowed for confidential transfers, can be toggled by the governance
 module. Withdrawals are always allowed, even when this is set to <code><b>false</b></code>.
 If <code>FAController::allow_list_enabled</code> is <code><b>false</b></code>, all asset types are allowed, even if this is <code><b>false</b></code>.
</dd>
<dt>
<code>auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>
 The auditor's public key for the asset type. If the auditor is not set, this field is <code>None</code>.
 Otherwise, each confidential transfer must include the auditor as an additional party,
 alongside the recipient, who has access to the decrypted transferred amount.

 TODO(feature): add global auditor EK too
 TODO(feature): add support for multiple auditors here
</dd>
</dl>


</details>

<a id="0x7_confidential_asset_Deposited"></a>

## Struct `Deposited`

Emitted when someone brings confidential assets into the protocol via <code>deposit_to</code>: i.e., by depositing a fungible
asset into the "confidential pool" and minting a confidential asset as "proof" of this.


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="confidential_asset.md#0x7_confidential_asset_Deposited">Deposited</a> <b>has</b> drop, store
</code></pre>



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
<code>asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_asset_Withdrawn"></a>

## Struct `Withdrawn`

Emitted when someone brings confidential assets out of the protocol via <code>withdraw_to</code>: i.e., by burning a confidential
asset as "proof" of being allowed to withdraw a fungible asset from the "confidential pool."


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="confidential_asset.md#0x7_confidential_asset_Withdrawn">Withdrawn</a> <b>has</b> drop, store
</code></pre>



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
<code>asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_asset_Transferred"></a>

## Struct `Transferred`

Emitted when confidential assets are transferred within the protocol between users' confidential balances.
Note that a numeric amount is not included, as the whole point of the protocol is to avoid leaking it.


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
<b>struct</b> <a href="confidential_asset.md#0x7_confidential_asset_Transferred">Transferred</a> <b>has</b> drop, store
</code></pre>



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
<code>asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_confidential_asset_KeyRotationProof"></a>

## Enum `KeyRotationProof`

TODO: Move this up at some point


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_KeyRotationProof">KeyRotationProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>compressed_new_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_confidential_asset_E_ALREADY_NORMALIZED"></a>

The balance is already normalized and cannot be normalized again.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_ALREADY_NORMALIZED">E_ALREADY_NORMALIZED</a>: u64 = 8;
</code></pre>



<a id="0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED"></a>

The asset type is currently not allowed for confidential transfers.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>: u64 = 9;
</code></pre>



<a id="0x7_confidential_asset_E_CONFIDENTIAL_STORE_ALREADY_REGISTERED"></a>

The confidential store has already been published for the given user and asset-type pair: user need not call <code>register</code> again.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_ALREADY_REGISTERED">E_CONFIDENTIAL_STORE_ALREADY_REGISTERED</a>: u64 = 2;
</code></pre>



<a id="0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED"></a>

The confidential store has not been published for the given user and asset-type pair: user should call <code>register</code>.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>: u64 = 3;
</code></pre>



<a id="0x7_confidential_asset_E_INCOMING_TRANSFERS_NOT_PAUSED"></a>

Incoming transfers must be paused before key rotation.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_NOT_PAUSED">E_INCOMING_TRANSFERS_NOT_PAUSED</a>: u64 = 10;
</code></pre>



<a id="0x7_confidential_asset_E_INCOMING_TRANSFERS_PAUSED"></a>

Incoming transfers must NOT be paused before depositing or receiving a transfer.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>: u64 = 4;
</code></pre>



<a id="0x7_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET"></a>


<a id="@[test_only]_The_confidential_asset_module_initialization_failed._1"></a>

### [test_only] The confidential asset module initialization failed.



<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>: u64 = 1000;
</code></pre>



<a id="0x7_confidential_asset_E_INTERNAL_ERROR"></a>

An internal error occurred: there is either a bug or a misconfiguration in the contract.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>: u64 = 999;
</code></pre>



<a id="0x7_confidential_asset_E_NORMALIZATION_REQUIRED"></a>

The available balance must be normalized before roll-over to ensure available balance chunks remain 32-bit after.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_NORMALIZATION_REQUIRED">E_NORMALIZATION_REQUIRED</a>: u64 = 7;
</code></pre>



<a id="0x7_confidential_asset_E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE"></a>

No user has deposited this asset type yet into their confidential store.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE">E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE</a>: u64 = 11;
</code></pre>



<a id="0x7_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER"></a>

The receiver's pending balance has accumulated too many incoming transferes and must be rolled over into the available balance.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>: u64 = 6;
</code></pre>



<a id="0x7_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION"></a>

The pending balance must be zero before rotating the encryption key.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>: u64 = 5;
</code></pre>



<a id="0x7_confidential_asset_E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE"></a>

The range proof system does not support sufficient range.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>: u64 = 1;
</code></pre>



<a id="0x7_confidential_asset_MAINNET_CHAIN_ID"></a>

The mainnet chain ID. If the chain ID is 1, the allow list is enabled.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_MAINNET_CHAIN_ID">MAINNET_CHAIN_ID</a>: u8 = 1;
</code></pre>



<a id="0x7_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER"></a>

The maximum number of transactions can be aggregated on the pending balance before rollover is required.
i.e., <code>ConfidentialStore::transfers_received</code> will never exceed this value.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>: u64 = 65536;
</code></pre>



<a id="0x7_confidential_asset_TESTNET_CHAIN_ID"></a>

The testnet chain ID.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_TESTNET_CHAIN_ID">TESTNET_CHAIN_ID</a>: u8 = 2;
</code></pre>



<a id="0x7_confidential_asset_init_module"></a>

## Function `init_module`

Called only once, when this module is first published on the blockchain.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_init_module">init_module</a>(deployer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_init_module">init_module</a>(deployer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    // TODO: Just asserting <b>if</b> my understanding is correct that `deployer == @aptos_experimental`
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer) == @aptos_experimental, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>));

    <b>assert</b>!(
        bulletproofs::get_max_range_bits()
            &gt;= <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_num_bits">confidential_proof::get_bulletproofs_num_bits</a>(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>)
    );

    <b>let</b> deployer_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer);

    <b>move_to</b>(
        deployer,
        <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
            allow_list_enabled: <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="confidential_asset.md#0x7_confidential_asset_MAINNET_CHAIN_ID">MAINNET_CHAIN_ID</a>,
            // DO NOT CHANGE: using long syntax until framework change is released <b>to</b> mainnet
            extend_ref: <a href="../../aptos-framework/doc/object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(&<a href="../../aptos-framework/doc/object.md#0x1_object_create_object">object::create_object</a>(deployer_address))
        }
    );
}
</code></pre>



</details>

<a id="0x7_confidential_asset_init_module_for_devnet"></a>

## Function `init_module_for_devnet`

Used to initialize the module for devnet and for tests in aptos-move/e2e-move-tests/


<pre><code>entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_init_module_for_devnet">init_module_for_devnet</a>(deployer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_init_module_for_devnet">init_module_for_devnet</a>(deployer: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer) == @aptos_experimental,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id_get">chain_id::get</a>() != <a href="confidential_asset.md#0x7_confidential_asset_MAINNET_CHAIN_ID">MAINNET_CHAIN_ID</a>,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id_get">chain_id::get</a>() != <a href="confidential_asset.md#0x7_confidential_asset_TESTNET_CHAIN_ID">TESTNET_CHAIN_ID</a>,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INIT_MODULE_FAILED_FOR_DEVNET">E_INIT_MODULE_FAILED_FOR_DEVNET</a>)
    );

    <a href="confidential_asset.md#0x7_confidential_asset_init_module">init_module</a>(deployer)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_register"></a>

## Function `register`

Registers an account for a specified asset type.
TODO: make it independent of the asset type. the "confidential store", if non existant, can be created at receiving time
TODO(Security): ZKPoK of DK

Users are also responsible for generating a Twisted ElGamal key pair on their side.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register">register</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register">register</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> {
    <b>let</b> ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(ek).extract();

    <a href="confidential_asset.md#0x7_confidential_asset_register_internal">register_internal</a>(sender, asset_type, ek);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_deposit_to"></a>

## Function `deposit_to`

Brings tokens into the protocol, transferring the passed amount from the sender's primary FA store
to the pending balance of the recipient.
The initial confidential balance is publicly visible, as entering the protocol requires a normal transfer.
However, tokens within the protocol become obfuscated through confidential transfers, ensuring privacy in
subsequent transactions.
TODO: grieving attack so remove


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit_to">deposit_to</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit_to">deposit_to</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_deposit_to_internal">deposit_to_internal</a>(sender, asset_type, <b>to</b>, amount)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_deposit"></a>

## Function `deposit`

The same as <code>deposit_to</code>, but the recipient is the sender.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit">deposit</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit">deposit</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_deposit_to_internal">deposit_to_internal</a>(
        sender,
        asset_type,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        amount
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_withdraw_to"></a>

## Function `withdraw_to`

Brings tokens out of the protocol by transferring the specified amount from the sender's available balance to
the recipient's primary FA store.
The withdrawn amount is publicly visible, as this process requires a normal transfer.
The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to">withdraw_to</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64, new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to">withdraw_to</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64,
    new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> new_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">confidential_balance::new_balance_from_bytes</a>(new_balance, get_num_available_chunks()).extract();
    <b>let</b> proof =
        <a href="confidential_proof.md#0x7_confidential_proof_deserialize_withdrawal_proof">confidential_proof::deserialize_withdrawal_proof</a>(sigma_proof, zkrp_new_balance)
            .extract();

    <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to_internal">withdraw_to_internal</a>(sender, asset_type, <b>to</b>, amount, new_balance, proof);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_withdraw"></a>

## Function `withdraw`

The same as <code>withdraw_to</code>, but the recipient is the sender.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw">withdraw</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64, new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw">withdraw</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    amount: u64,
    new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to">withdraw_to</a>(
        sender,
        asset_type,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        amount,
        new_balance,
        zkrp_new_balance,
        sigma_proof
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_confidential_transfer"></a>

## Function `confidential_transfer`

Transfers tokens from the sender's available balance to the recipient's pending balance.
The function hides the transferred amount while keeping the sender and recipient addresses visible.
The sender encrypts the transferred amount with the recipient's encryption key and the function updates the
recipient's confidential balance homomorphically.
Additionally, the sender encrypts the transferred amount with the auditors' EKs, allowing auditors to decrypt
it on their side.
The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.
Warning: If the auditor feature is enabled, the sender must include the auditor as the first element in the
<code>auditor_eks</code> vector.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer">confidential_transfer</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sender_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, auditor_amounts: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_transfer_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer">confidential_transfer</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sender_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recipient_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    auditor_amounts: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_transfer_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> new_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">confidential_balance::new_balance_from_bytes</a>(new_balance, get_num_available_chunks()).extract();
    <b>let</b> sender_amount =
        <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">confidential_balance::new_balance_from_bytes</a>(sender_amount, get_num_pending_chunks()).extract();
    <b>let</b> recipient_amount =
        <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">confidential_balance::new_balance_from_bytes</a>(recipient_amount, get_num_pending_chunks()).extract();
    <b>let</b> auditor_eks = <a href="confidential_asset.md#0x7_confidential_asset_deserialize_auditor_eks">deserialize_auditor_eks</a>(auditor_eks).extract();
    <b>let</b> auditor_amounts = <a href="confidential_asset.md#0x7_confidential_asset_deserialize_auditor_amounts">deserialize_auditor_amounts</a>(auditor_amounts).extract();
    <b>let</b> proof =
        <a href="confidential_proof.md#0x7_confidential_proof_deserialize_transfer_proof">confidential_proof::deserialize_transfer_proof</a>(
            sigma_proof, zkrp_new_balance, zkrp_transfer_amount
        ).extract();

    <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer_internal">confidential_transfer_internal</a>(
        sender,
        asset_type,
        <b>to</b>,
        new_balance,
        sender_amount,
        recipient_amount,
        auditor_eks,
        auditor_amounts,
        proof
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rotate_encryption_key"></a>

## Function `rotate_encryption_key`

Rotates the encryption key for the user's confidential balance, updating it to a new encryption key.
Parses arguments and forwards to <code>rotate_encryption_key_internal</code>; see that function for details.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, resume_incoming_transfers: bool, new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    resume_incoming_transfers: bool,
    new_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
    sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
    sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    // Just parse stuff and forward <b>to</b> the more type-safe function
    <b>let</b> (new_ek, compressed_new_ek) = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_and_compressed_from_bytes">ristretto255::new_point_and_compressed_from_bytes</a>(new_ek);
    <b>let</b> (new_D, compressed_new_D) = <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_deserialize_points">sigma_protocol_utils::deserialize_points</a>(new_D);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(
        sigma_proto_comm, sigma_proto_resp
    );

    <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key_internal">rotate_encryption_key_internal</a>(
        sender, asset_type, new_ek,
        KeyRotationProof::V1 { compressed_new_ek, new_D, compressed_new_D, sigma },
        resume_incoming_transfers
    );
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rotate_encryption_key_internal"></a>

## Function `rotate_encryption_key_internal`

TODO(Comment): add comments explaining the parameters


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key_internal">rotate_encryption_key_internal</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="confidential_asset.md#0x7_confidential_asset_KeyRotationProof">confidential_asset::KeyRotationProof</a>, resume_incoming_transfers: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key_internal">rotate_encryption_key_internal</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_ek: RistrettoPoint,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_KeyRotationProof">KeyRotationProof</a>,
    resume_incoming_transfers: bool,
) {
    //
    // Step 1: Safety-checks that (1) incoming transfers are paused and (2) pending balance is zero because it <b>has</b>
    //         been rolled over
    //

    <b>let</b> ca_store = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(
        <a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type)
    );

    // (1) Assert incoming transfers are paused & unpause them after <b>if</b> flag is set maybe
    <b>assert</b>!(ca_store.pause_incoming_transfers, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_NOT_PAUSED">E_INCOMING_TRANSFERS_NOT_PAUSED</a>));

    // (2) Assert that the pending balance is zero before rotating the key. The user must call `rollover_pending_balance`
    // before rotating their key <b>with</b> `pause` set <b>to</b> `<b>true</b>`.
    <b>assert</b>!(
        <a href="confidential_balance.md#0x7_confidential_balance_is_zero_balance">confidential_balance::is_zero_balance</a>(&ca_store.pending_balance),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>)
    );
    // Over-asserting invariants, in an abundance of caution.
    <b>assert</b>!(
        ca_store.transfers_received == 0,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>)
    );

    //
    // Step 2: Fetch <b>old</b> available balance and the <b>old</b> EK from on-chain
    //

    <b>let</b> compressed_H = <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed</a>();
    <b>let</b> compressed_old_ek = ca_store.ek;
    <b>let</b> old_ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&compressed_old_ek);
    <b>let</b> compressed_old_D = *ca_store.available_balance.get_compressed_D();
    <b>let</b> old_D = <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils_decompress_points">sigma_protocol_utils::decompress_points</a>(&compressed_old_D);

    //
    // Step 3: Verify the Sigma protocol proof of correct re-encryption
    //
    <b>let</b> ss = <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_session">sigma_protocol_key_rotation::new_session</a>(owner, asset_type, get_num_available_chunks());
    <b>let</b> KeyRotationProof::V1 { compressed_new_ek, new_D, compressed_new_D, sigma } =  proof;
    // Note: Will check that compressed_old_D.length() == compressed_new_D.length() == num_chunks &gt; 0
    <b>let</b> stmt = <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_key_rotation_statement">sigma_protocol_key_rotation::new_key_rotation_statement</a>(
        // TODO(Perf): Can we avoid the expensive decompression of `H`? (May need a <b>native</b> function.)
        compressed_H, <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&compressed_H),
        compressed_old_ek, old_ek,
        compressed_new_ek, new_ek,
        compressed_old_D, old_D,
        compressed_new_D, new_D,
        get_num_available_chunks(),
    );
    <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_verifies">sigma_protocol_key_rotation::assert_verifies</a>(&ss, &stmt, &sigma, get_num_available_chunks());

    //
    // Step 4: Install the new EK and the new re-encrypted available balance
    //
    ca_store.ek = compressed_new_ek;
    // Note: The pending balance <b>has</b> been asserted <b>to</b> be zero. We're just updating the available balance.
    // The C components stay the same (they don't depend on the EK); only D = r * EK changes.
    ca_store.available_balance.set_compressed_D(compressed_new_D);

    // Note: ca_store.pause_incoming_transfers is already set <b>to</b> `<b>true</b>`
    <b>if</b> (resume_incoming_transfers) {
        ca_store.pause_incoming_transfers = <b>false</b>;
    }
}
</code></pre>



</details>

<a id="0x7_confidential_asset_normalize"></a>

## Function `normalize`

Adjusts each chunk to fit into defined 16-bit bounds to prevent overflows.
Most functions perform implicit normalization by accepting a new normalized confidential balance as a parameter.
However, explicit normalization is required before rolling over the pending balance, as multiple rolls may cause
chunk overflows.
The sender provides their new normalized confidential balance, encrypted with fresh randomness to preserve privacy.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize">normalize</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize">normalize</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proof: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> new_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">confidential_balance::new_balance_from_bytes</a>(new_balance, get_num_available_chunks()).extract();
    <b>let</b> proof =
        <a href="confidential_proof.md#0x7_confidential_proof_deserialize_normalization_proof">confidential_proof::deserialize_normalization_proof</a>(
            sigma_proof, zkrp_new_balance
        ).extract();

    <a href="confidential_asset.md#0x7_confidential_asset_normalize_internal">normalize_internal</a>(sender, asset_type, new_balance, proof);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_pause_incoming_transactions"></a>

## Function `pause_incoming_transactions`

Pauses receiving incoming transfers for the specified account and asset type.
Needed for one scenario:
1. Before rotating their encryption key, the owner must pause incoming transfers so as to be able to roll over
their pending balance fully. Then, to rotate their encryption key, the owner needs to only re-encrypt their
available balance ciphertext. Once done, the owner can unpause incoming transfers.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_pause_incoming_transactions">pause_incoming_transactions</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_pause_incoming_transactions">pause_incoming_transactions</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_pause_incoming_transactions_internal">pause_incoming_transactions_internal</a>(owner, asset_type);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_resume_incoming_transactions"></a>

## Function `resume_incoming_transactions`

Allows receiving incoming transfers for the specified account and asset type.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_resume_incoming_transactions">resume_incoming_transactions</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_resume_incoming_transactions">resume_incoming_transactions</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_resume_incoming_transactions_internal">resume_incoming_transactions_internal</a>(owner, asset_type);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rollover_pending_balance"></a>

## Function `rollover_pending_balance`

Adds the pending balance to the available balance for the specified asset type, resetting the pending balance to zero.
This operation is needed when the owner wants to be able to send out tokens from their pending balance: the only
way of doing so is to roll over these tokens into the available balance.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance">rollover_pending_balance</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance">rollover_pending_balance</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_internal">rollover_pending_balance_internal</a>(sender, asset_type);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rollover_pending_balance_and_freeze"></a>

## Function `rollover_pending_balance_and_freeze`

Before calling <code>rotate_encryption_key</code>, we need to rollover the pending balance and freeze the asset type to
prevent any new transfers from coming in.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_and_freeze">rollover_pending_balance_and_freeze</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_and_freeze">rollover_pending_balance_and_freeze</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance">rollover_pending_balance</a>(sender, asset_type);
    <a href="confidential_asset.md#0x7_confidential_asset_pause_incoming_transactions">pause_incoming_transactions</a>(sender, asset_type);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_enable_allow_listing"></a>

## Function `enable_allow_listing`

Enables the allow list, restricting confidential transfers to asset types on the allow list.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_enable_allow_listing">enable_allow_listing</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_enable_allow_listing">enable_allow_listing</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> fa_controller = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>&gt;(@aptos_experimental);
    fa_controller.allow_list_enabled = <b>true</b>;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_disable_allow_listing"></a>

## Function `disable_allow_listing`

Disables the allow list, allowing confidential transfers for all asset types.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_disable_allow_listing">disable_allow_listing</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_disable_allow_listing">disable_allow_listing</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> fa_controller = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>&gt;(@aptos_experimental);
    fa_controller.allow_list_enabled = <b>false</b>;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_enable_confidentiality_for_asset_type"></a>

## Function `enable_confidentiality_for_asset_type`

Enables confidential transfers for the specified asset type.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_enable_confidentiality_for_asset_type">enable_confidentiality_for_asset_type</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_enable_confidentiality_for_asset_type">enable_confidentiality_for_asset_type</a>(
    aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> fa_config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address_or_create">get_fa_config_address_or_create</a>(asset_type));
    fa_config.allowed = <b>true</b>;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_disable_confidentiality_for_asset_type"></a>

## Function `disable_confidentiality_for_asset_type`

Disables confidential transfers for the specified asset type.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_disable_confidentiality_for_asset_type">disable_confidentiality_for_asset_type</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_disable_confidentiality_for_asset_type">disable_confidentiality_for_asset_type</a>(
    aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> fa_config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address_or_create">get_fa_config_address_or_create</a>(asset_type));
    fa_config.allowed = <b>false</b>;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_set_auditor_for_asset_type"></a>

## Function `set_auditor_for_asset_type`

Sets the auditor for the specified asset type.

NOTE: Ensures that new_auditor_ek is a valid Ristretto255 point
TODO(Security): ZKPoK of DK?


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_auditor_for_asset_type">set_auditor_for_asset_type</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_auditor_for_asset_type">set_auditor_for_asset_type</a>(
    aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> fa_config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address_or_create">get_fa_config_address_or_create</a>(asset_type));
    fa_config.auditor_ek = std::option::some(<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(auditor_ek).extract());
}
</code></pre>



</details>

<a id="0x7_confidential_asset_set_auditor_globally"></a>

## Function `set_auditor_globally`

Sets the global auditor for all asset types.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_auditor_globally">set_auditor_globally</a>(_aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_auditor_globally">set_auditor_globally</a>(_aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    // TODO: Implement
}
</code></pre>



</details>

<a id="0x7_confidential_asset_has_confidential_store"></a>

## Function `has_confidential_store`

Checks if the user has a confidential store for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): bool {
    <b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_is_confidentiality_enabled_for_asset_type"></a>

## Function `is_confidentiality_enabled_for_asset_type`

Returns true if confidentiality is enabled for all assets or if the asset type is allowed for confidential
transfers. Returns false otherwise.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> {
    <b>if</b> (!<a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>()) {
        <b>return</b> <b>true</b>
    };

    <b>let</b> fa_config_address = <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address">get_fa_config_address</a>(asset_type);

    <b>if</b> (!<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(fa_config_address)) {
        <b>return</b> <b>false</b>
    };

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(fa_config_address).allowed
}
</code></pre>



</details>

<a id="0x7_confidential_asset_is_allow_listing_enabled"></a>

## Function `is_allow_listing_enabled`

Checks if allow listing is enabled.
If the allow list is enabled, only asset types from the allow list can be transferred confidentially.
Otherwise, all asset types are allowed.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>(): bool <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>&gt;(@aptos_experimental).allow_list_enabled
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_pending_balance"></a>

## Function `get_pending_balance`

Returns the pending balance of the user for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_pending_balance">get_pending_balance</a>(owner: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_pending_balance">get_pending_balance</a>(
    owner: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(owner, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>let</b> ca_store =
        <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(owner, asset_type));

    ca_store.pending_balance
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_available_balance"></a>

## Function `get_available_balance`

Returns the available balance of the user for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_available_balance">get_available_balance</a>(owner: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_available_balance">get_available_balance</a>(
    owner: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): <a href="confidential_balance.md#0x7_confidential_balance_CompressedConfidentialBalance">confidential_balance::CompressedConfidentialBalance</a> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(owner, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>let</b> ca_store =
        <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(owner, asset_type));

    ca_store.available_balance
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_encryption_key"></a>

## Function `get_encryption_key`

Returns the encryption key (EK) of the user for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): CompressedRistretto <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type)).ek
}
</code></pre>



</details>

<a id="0x7_confidential_asset_is_normalized"></a>

## Function `is_normalized`

Checks if the user's available balance is normalized for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_normalized">is_normalized</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_normalized">is_normalized</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): bool <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type)).normalized
}
</code></pre>



</details>

<a id="0x7_confidential_asset_incoming_transfers_paused"></a>

## Function `incoming_transfers_paused`

Checks if the user's incoming transfers are paused for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type)).pause_incoming_transfers
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_auditor_for_asset_type"></a>

## Function `get_auditor_for_asset_type`

Returns the asset-specific auditor's encryption key.
If the auditing feature is disabled for the asset type, the encryption key is set to <code>None</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_auditor_for_asset_type">get_auditor_for_asset_type</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_auditor_for_asset_type">get_auditor_for_asset_type</a>(
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): Option&lt;CompressedRistretto&gt; <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> fa_config_address = <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address">get_fa_config_address</a>(asset_type);

    <b>if</b> (!<a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>() && !<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(fa_config_address)) {
        <b>return</b> std::option::none();
    };

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(fa_config_address).auditor_ek
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_total_supply"></a>

## Function `get_total_supply`

Returns the circulating supply of the confidential asset.
TODO: rename to get_total_confidential_supply


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_total_supply">get_total_supply</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_total_supply">get_total_supply</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64 <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> fa_store_address = <a href="confidential_asset.md#0x7_confidential_asset_get_fa_controller_address">get_fa_controller_address</a>();
    <b>assert</b>!(
        <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_primary_store_exists">primary_fungible_store::primary_store_exists</a>(fa_store_address, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE">E_NO_CONFIDENTIAL_ASSET_POOL_FOR_ASSET_TYPE</a>)
    );

    <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_balance">primary_fungible_store::balance</a>(fa_store_address, asset_type)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_num_transfers_received"></a>

## Function `get_num_transfers_received`

Returns the number of transfers received into the pending balance for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_num_transfers_received">get_num_transfers_received</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_num_transfers_received">get_num_transfers_received</a>(
    user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): u64 <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type)).transfers_received
}
</code></pre>



</details>

<a id="0x7_confidential_asset_register_internal"></a>

## Function `register_internal`

Implementation of the <code>register</code> entry function.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register_internal">register_internal</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register_internal">register_internal</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: CompressedRistretto
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));

    <b>let</b> user = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    <b>assert</b>!(
        !<a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_ALREADY_REGISTERED">E_CONFIDENTIAL_STORE_ALREADY_REGISTERED</a>)
    );

    <b>let</b> ca_store = <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
        pause_incoming_transfers: <b>false</b>,
        normalized: <b>true</b>,
        transfers_received: 0,
        pending_balance: <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_zero_balance">confidential_balance::new_compressed_zero_balance</a>(get_num_pending_chunks()),
        available_balance: <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_zero_balance">confidential_balance::new_compressed_zero_balance</a>(get_num_available_chunks()),
        ek
    };

    <b>move_to</b>(&<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_signer">get_confidential_store_signer</a>(sender, asset_type), ca_store);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_deposit_to_internal"></a>

## Function `deposit_to_internal`

Implementation of the <code>deposit_to</code> entry function.
For convenience, we often refer to this operation as "veiling."
TODO: remove ability to deposit to another's account


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit_to_internal">deposit_to_internal</a>(depositor: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit_to_internal">deposit_to_internal</a>(
    depositor: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));
    <b>assert</b>!(!<a href="confidential_asset.md#0x7_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(<b>to</b>, asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>));

    <b>let</b> depositor_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(depositor);

    <b>let</b> depositor_fa_store = <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_fungible_store::primary_store</a>(depositor_addr, asset_type);

    // Note: This sets up the "confidential asset pool" for this asset type, <b>if</b> one is not already set up, such <b>as</b>
    // when someone first veils this asset type for the first time.
    <b>let</b> pool_fa_store =
        <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(
            <a href="confidential_asset.md#0x7_confidential_asset_get_fa_controller_address">get_fa_controller_address</a>(), asset_type
        );

    //
    // Step 1: Transfer the asset from the user's <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> into the confidential asset pool
    //
    <a href="../../aptos-framework/doc/dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">dispatchable_fungible_asset::transfer</a>(
        depositor, depositor_fa_store, pool_fa_store, amount
    );

    //
    // Step 2: "Mint" correspodning confidential assets for the depositor, and add them <b>to</b> their pending balance.
    //
    <b>let</b> depositor_ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(<b>to</b>, asset_type));

    // Make sure the receiver <b>has</b> "room" in their pending balance for this deposit
    <b>assert</b>!(
        depositor_ca_store.transfers_received &lt; <a href="confidential_asset.md#0x7_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>)
    );

    <b>let</b> pending_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_decompress">confidential_balance::decompress</a>(&depositor_ca_store.pending_balance);

    <a href="confidential_balance.md#0x7_confidential_balance_add_balances_mut">confidential_balance::add_balances_mut</a>(
        &<b>mut</b> pending_balance,
        &<a href="confidential_balance.md#0x7_confidential_balance_new_pending_balance_u64_no_randomness">confidential_balance::new_pending_balance_u64_no_randomness</a>(amount)
    );

    // Update the pending balance and increment the incoming transfers counter
    depositor_ca_store.pending_balance = <a href="confidential_balance.md#0x7_confidential_balance_compress">confidential_balance::compress</a>(&pending_balance);
    depositor_ca_store.transfers_received += 1;

    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="confidential_asset.md#0x7_confidential_asset_Deposited">Deposited</a> { from: depositor_addr, <b>to</b>, amount, asset_type });
}
</code></pre>



</details>

<a id="0x7_confidential_asset_withdraw_to_internal"></a>

## Function `withdraw_to_internal`

Implementation of the <code>withdraw_to</code> entry function.
Withdrawals are always allowed, regardless of whether the asset type is allow-listed.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to_internal">withdraw_to_internal</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64, new_balance: <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: <a href="confidential_proof.md#0x7_confidential_proof_WithdrawalProof">confidential_proof::WithdrawalProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to_internal">withdraw_to_internal</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64,
    new_balance: ConfidentialBalance,
    proof: WithdrawalProof
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> from = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    <b>let</b> sender_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(from, asset_type);

    <b>let</b> ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(from, asset_type));
    <b>let</b> current_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_decompress">confidential_balance::decompress</a>(&ca_store.available_balance);

    <a href="confidential_proof.md#0x7_confidential_proof_verify_withdrawal_proof">confidential_proof::verify_withdrawal_proof</a>(
        &sender_ek,
        amount,
        &current_balance,
        &new_balance,
        &proof
    );

    ca_store.normalized = <b>true</b>;
    ca_store.available_balance = <a href="confidential_balance.md#0x7_confidential_balance_compress">confidential_balance::compress</a>(&new_balance);

    <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_transfer">primary_fungible_store::transfer</a>(&<a href="confidential_asset.md#0x7_confidential_asset_get_fa_controller_signer">get_fa_controller_signer</a>(), asset_type, <b>to</b>, amount);

    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="confidential_asset.md#0x7_confidential_asset_Withdrawn">Withdrawn</a> { from: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), <b>to</b>, amount, asset_type });
}
</code></pre>



</details>

<a id="0x7_confidential_asset_confidential_transfer_internal"></a>

## Function `confidential_transfer_internal`

Implementation of the <code>confidential_transfer</code> entry function.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer_internal">confidential_transfer_internal</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, new_balance: <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, sender_amount: <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, recipient_amount: <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, auditor_amounts: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;, proof: <a href="confidential_proof.md#0x7_confidential_proof_TransferProof">confidential_proof::TransferProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer_internal">confidential_transfer_internal</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    new_balance: ConfidentialBalance,
    sender_amount: ConfidentialBalance,
    recipient_amount: ConfidentialBalance,
    auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    auditor_amounts: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ConfidentialBalance&gt;,
    proof: TransferProof
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));
    <b>assert</b>!(!<a href="confidential_asset.md#0x7_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(<b>to</b>, asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>));
    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_validate_auditors">validate_auditors</a>(
            asset_type,
            &recipient_amount,
            &auditor_eks,
            &auditor_amounts,
            &proof
        ),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>) // TODO: i removed the <b>old</b> <a href="../../aptos-framework/doc/code.md#0x1_code">code</a> because <a href="confidential_asset.md#0x7_confidential_asset_validate_auditors">validate_auditors</a>() will be turned into fetch_auditor_eks()
    );

    // TODO: This will be removed when we build the more efficient $\Sigma$-protocol
    <b>assert</b>!(
        <a href="confidential_balance.md#0x7_confidential_balance_balance_c_equals">confidential_balance::balance_c_equals</a>(&sender_amount, &recipient_amount),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>)   // note: i removed the <b>old</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>
    );

    <b>let</b> from = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    <b>let</b> sender_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(from, asset_type);
    <b>let</b> recipient_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(<b>to</b>, asset_type);

    <b>let</b> sender_ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(from, asset_type));

    <b>let</b> sender_current_available_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_decompress">confidential_balance::decompress</a>(&sender_ca_store.available_balance);

    <a href="confidential_proof.md#0x7_confidential_proof_verify_transfer_proof">confidential_proof::verify_transfer_proof</a>(
        &sender_ek,
        &recipient_ek,
        &sender_current_available_balance,
        &new_balance,
        &sender_amount,
        &recipient_amount,
        &auditor_eks,
        &auditor_amounts,
        &proof
    );

    sender_ca_store.normalized = <b>true</b>;
    sender_ca_store.available_balance = <a href="confidential_balance.md#0x7_confidential_balance_compress">confidential_balance::compress</a>(
        &new_balance
    );

    // Cannot create multiple mutable references <b>to</b> the same type, so we need <b>to</b> drop it
    <b>let</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> { .. } = sender_ca_store;

    <b>let</b> recipient_ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(<b>to</b>, asset_type));

    // Make sure the receiver <b>has</b> "room" in their pending balance for this transfer
    <b>assert</b>!(
        recipient_ca_store.transfers_received &lt; <a href="confidential_asset.md#0x7_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>)
    );

    <b>let</b> recipient_pending_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_decompress">confidential_balance::decompress</a>(
            &recipient_ca_store.pending_balance
        );
    <a href="confidential_balance.md#0x7_confidential_balance_add_balances_mut">confidential_balance::add_balances_mut</a>(
        &<b>mut</b> recipient_pending_balance, &recipient_amount
    );

    recipient_ca_store.transfers_received += 1;
    recipient_ca_store.pending_balance = <a href="confidential_balance.md#0x7_confidential_balance_compress">confidential_balance::compress</a>(
        &recipient_pending_balance
    );

    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(<a href="confidential_asset.md#0x7_confidential_asset_Transferred">Transferred</a> { from, <b>to</b>, asset_type });
}
</code></pre>



</details>

<a id="0x7_confidential_asset_normalize_internal"></a>

## Function `normalize_internal`

Implementation of the <code>normalize</code> entry function.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize_internal">normalize_internal</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_balance: <a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, proof: <a href="confidential_proof.md#0x7_confidential_proof_NormalizationProof">confidential_proof::NormalizationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize_internal">normalize_internal</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_balance: ConfidentialBalance,
    proof: NormalizationProof
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> user = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>let</b> sender_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(user, asset_type);

    <b>let</b> ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type));

    <b>assert</b>!(!ca_store.normalized, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ALREADY_NORMALIZED">E_ALREADY_NORMALIZED</a>));

    <b>let</b> current_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_decompress">confidential_balance::decompress</a>(&ca_store.available_balance);

    <a href="confidential_proof.md#0x7_confidential_proof_verify_normalization_proof">confidential_proof::verify_normalization_proof</a>(
        &sender_ek,
        &current_balance,
        &new_balance,
        &proof
    );

    ca_store.available_balance = <a href="confidential_balance.md#0x7_confidential_balance_compress">confidential_balance::compress</a>(&new_balance);
    ca_store.normalized = <b>true</b>;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rollover_pending_balance_internal"></a>

## Function `rollover_pending_balance_internal`

Implementation of the <code>rollover_pending_balance</code> entry function.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_internal">rollover_pending_balance_internal</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_internal">rollover_pending_balance_internal</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> user = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>let</b> ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type));

    <b>assert</b>!(ca_store.normalized, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_NORMALIZATION_REQUIRED">E_NORMALIZATION_REQUIRED</a>));

    <b>let</b> available_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_decompress">confidential_balance::decompress</a>(&ca_store.available_balance);
    <b>let</b> pending_balance =
        <a href="confidential_balance.md#0x7_confidential_balance_decompress">confidential_balance::decompress</a>(&ca_store.pending_balance);

    <a href="confidential_balance.md#0x7_confidential_balance_add_balances_mut">confidential_balance::add_balances_mut</a>(&<b>mut</b> available_balance, &pending_balance);

    ca_store.normalized = <b>false</b>;
    ca_store.transfers_received = 0;
    ca_store.available_balance = <a href="confidential_balance.md#0x7_confidential_balance_compress">confidential_balance::compress</a>(&available_balance);
    ca_store.pending_balance = <a href="confidential_balance.md#0x7_confidential_balance_new_compressed_zero_balance">confidential_balance::new_compressed_zero_balance</a>(get_num_pending_chunks());
}
</code></pre>



</details>

<a id="0x7_confidential_asset_pause_incoming_transactions_internal"></a>

## Function `pause_incoming_transactions_internal`

Implementation of the <code>pause_incoming_transactions</code> entry function.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_pause_incoming_transactions_internal">pause_incoming_transactions_internal</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_pause_incoming_transactions_internal">pause_incoming_transactions_internal</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> user = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>let</b> ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type));
    ca_store.pause_incoming_transfers = <b>true</b>;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_resume_incoming_transactions_internal"></a>

## Function `resume_incoming_transactions_internal`

Implementation of the <code>resume_incoming_transactions</code> entry function.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_resume_incoming_transactions_internal">resume_incoming_transactions_internal</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_resume_incoming_transactions_internal">resume_incoming_transactions_internal</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> user = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    <b>assert</b>!(
        <a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>)
    );

    <b>let</b> ca_store =
        <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type));
    ca_store.pause_incoming_transfers = <b>false</b>;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_fa_config_address"></a>

## Function `get_fa_config_address`

Returns the address that handles primary FA store and <code><a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a></code> objects for the specified asset type.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address">get_fa_config_address</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address">get_fa_config_address</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> fa_ext = &<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>&gt;(@aptos_experimental).extend_ref;
    <b>let</b> fa_ext_address = <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(fa_ext);
    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(&fa_ext_address, <a href="confidential_asset.md#0x7_confidential_asset_construct_fa_config_seed">construct_fa_config_seed</a>(asset_type))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_fa_config_address_or_create"></a>

## Function `get_fa_config_address_or_create`

Ensures that the <code><a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a></code> object exists for the specified asset type and returns its address.
If the object does not exist, creates it. Used only for internal purposes.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address_or_create">get_fa_config_address_or_create</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address_or_create">get_fa_config_address_or_create</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> addr = <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_address">get_fa_config_address</a>(asset_type);

    <b>if</b> (!<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>&gt;(addr)) {
        <b>let</b> fa_config_signer = <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_signer">get_fa_config_signer</a>(asset_type);

        <b>move_to</b>(
            &fa_config_signer,
            // We disallow the asset type from being made confidential since this function is
            // called in a lot of different contexts.
            <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a> { allowed: <b>false</b>, auditor_ek: std::option::none() }
        );
    };

    addr
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_fa_controller_signer"></a>

## Function `get_fa_controller_signer`

Returns an object for handling all the FA primary stores, and returns a signer for it.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_controller_signer">get_fa_controller_signer</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_controller_signer">get_fa_controller_signer</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>&gt;(@aptos_experimental).extend_ref)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_fa_controller_address"></a>

## Function `get_fa_controller_address`

Returns the address that handles all the FA primary stores.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_controller_address">get_fa_controller_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_controller_address">get_fa_controller_address</a>(): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(&<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>&gt;(@aptos_experimental).extend_ref)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_confidential_store_signer"></a>

## Function `get_confidential_store_signer`

Returns an object for handling the <code><a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a></code> and returns a signer for it.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_signer">get_confidential_store_signer</a>(user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_signer">get_confidential_store_signer</a>(user: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(&<a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(user, <a href="confidential_asset.md#0x7_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type)))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_confidential_store_address"></a>

## Function `get_confidential_store_address`

Returns the address that handles the user's <code><a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a></code> object for the specified user and asset type.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(&user, <a href="confidential_asset.md#0x7_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_fa_config_signer"></a>

## Function `get_fa_config_signer`

Returns an object for handling the <code><a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a></code>, and returns a signer for it.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_signer">get_fa_config_signer</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_fa_config_signer">get_fa_config_signer</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>let</b> fa_ext = &<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a>&gt;(@aptos_experimental).extend_ref;
    <b>let</b> fa_ext_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(fa_ext);

    <b>let</b> fa_ctor =
        &<a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(&fa_ext_signer, <a href="confidential_asset.md#0x7_confidential_asset_construct_fa_config_seed">construct_fa_config_seed</a>(asset_type));

    <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(fa_ctor)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_construct_confidential_store_seed"></a>

## Function `construct_confidential_store_seed`

Constructs a unique seed for the user's <code><a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a></code> object.
As all the <code><a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a></code>'s have the same type, we need to differentiate them by the seed.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_construct_confidential_store_seed">construct_confidential_store_seed</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(
        &<a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils_format2">string_utils::format2</a>(
            &b"<a href="confidential_asset.md#0x7_confidential_asset">confidential_asset</a>::{}::asset_type::{}::user",
            @aptos_experimental,
            <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&asset_type)
        )
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_construct_fa_config_seed"></a>

## Function `construct_fa_config_seed`

Constructs a unique seed for the FA's <code><a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a></code> object.
As all the <code><a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a></code>'s have the same type, we need to differentiate them by the seed.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_construct_fa_config_seed">construct_fa_config_seed</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_construct_fa_config_seed">construct_fa_config_seed</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(
        &<a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils_format2">string_utils::format2</a>(
            &b"<a href="confidential_asset.md#0x7_confidential_asset">confidential_asset</a>::{}::asset_type::{}::fa",
            @aptos_experimental,
            <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&asset_type)
        )
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_validate_auditors"></a>

## Function `validate_auditors`

Validates that the auditor-related fields in the confidential transfer are correct.
Returns <code><b>false</b></code> if the transfer amount is not the same as the auditor amounts.
Returns <code><b>false</b></code> if the number of auditors in the transfer proof and auditor lists do not match.
Returns <code><b>false</b></code> if the first auditor in the list and the asset-specific auditor do not match.
Note: If the asset-specific auditor is not set, the validation is successful for any list of auditors.
Otherwise, returns <code><b>true</b></code>.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_validate_auditors">validate_auditors</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, transfer_amount: &<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>, auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;, proof: &<a href="confidential_proof.md#0x7_confidential_proof_TransferProof">confidential_proof::TransferProof</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_validate_auditors">validate_auditors</a>(
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    transfer_amount: &ConfidentialBalance,
    auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    auditor_amounts: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ConfidentialBalance&gt;,
    proof: &TransferProof
): bool <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_FAConfig">FAConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_FAController">FAController</a> {
    <b>if</b> (!auditor_amounts.all(|auditor_amount| {
        <a href="confidential_balance.md#0x7_confidential_balance_balance_c_equals">confidential_balance::balance_c_equals</a>(transfer_amount, auditor_amount)
    })) {
        <b>return</b> <b>false</b>
    };

    <b>if</b> (auditor_eks.length() != auditor_amounts.length()
        || auditor_eks.length()
            != <a href="confidential_proof.md#0x7_confidential_proof_auditors_count_in_transfer_proof">confidential_proof::auditors_count_in_transfer_proof</a>(proof)) {
        <b>return</b> <b>false</b>
    };

    <b>let</b> asset_auditor_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_auditor_for_asset_type">get_auditor_for_asset_type</a>(asset_type);
    <b>if</b> (asset_auditor_ek.is_none()) {
        <b>return</b> <b>true</b>
    };

    <b>if</b> (auditor_eks.length() == 0) {
        <b>return</b> <b>false</b>
    };

    <b>let</b> asset_auditor_ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&asset_auditor_ek.extract());
    <b>let</b> first_auditor_ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_decompress">ristretto255::point_decompress</a>(&auditor_eks[0]);

    <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_point_equals">ristretto255::point_equals</a>(&asset_auditor_ek, &first_auditor_ek)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_deserialize_auditor_eks"></a>

## Function `deserialize_auditor_eks`

Deserializes the auditor EKs from a byte array.
Returns <code>Some(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;)</code> if the deserialization is successful, otherwise <code>None</code>.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deserialize_auditor_eks">deserialize_auditor_eks</a>(auditor_eks_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deserialize_auditor_eks">deserialize_auditor_eks</a>(
    auditor_eks_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): Option&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;&gt; {
    <b>if</b> (auditor_eks_bytes.length() % 32 != 0) {
        <b>return</b> std::option::none()
    };

    <b>let</b> auditors_count = auditor_eks_bytes.length() / 32;

    <b>let</b> auditor_eks = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, auditors_count).map(|i| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(
            auditor_eks_bytes.slice(i * 32, (i + 1) * 32)
        )
    });

    <b>if</b> (auditor_eks.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|ek| ek.is_none())) {
        <b>return</b> std::option::none()
    };

    std::option::some(auditor_eks.map(|ek| ek.extract()))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_deserialize_auditor_amounts"></a>

## Function `deserialize_auditor_amounts`

Deserializes the auditor amounts from a byte array.
Returns <code>Some(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ConfidentialBalance&gt;)</code> if the deserialization is successful, otherwise <code>None</code>.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deserialize_auditor_amounts">deserialize_auditor_amounts</a>(auditor_amounts_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_balance.md#0x7_confidential_balance_ConfidentialBalance">confidential_balance::ConfidentialBalance</a>&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deserialize_auditor_amounts">deserialize_auditor_amounts</a>(
    auditor_amounts_bytes: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): Option&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ConfidentialBalance&gt;&gt; {
    <b>if</b> (auditor_amounts_bytes.length() % 256 != 0) {
        <b>return</b> std::option::none()
    };

    <b>let</b> auditors_count = auditor_amounts_bytes.length() / 256;

    <b>let</b> auditor_amounts = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_range">vector::range</a>(0, auditors_count).map(|i| {
        <a href="confidential_balance.md#0x7_confidential_balance_new_balance_from_bytes">confidential_balance::new_balance_from_bytes</a>(
            auditor_amounts_bytes.slice(i * 256, (i + 1) * 256),
            get_num_pending_chunks()
        )
    });

    <b>if</b> (auditor_amounts.<a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>(|ek| ek.is_none())) {
        <b>return</b> std::option::none()
    };

    std::option::some(
        auditor_amounts.map(|balance| balance.extract())
    )
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
