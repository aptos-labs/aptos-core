
<a id="0x7_confidential_asset"></a>

# Module `0x7::confidential_asset`

This module implements the Confidential Asset (CA) Standard, a privacy-focused protocol for managing fungible assets (FA).
It enables private transfers by obfuscating transaction amounts while keeping sender and recipient addresses visible.


-  [Enum Resource `GlobalConfig`](#0x7_confidential_asset_GlobalConfig)
-  [Enum Resource `AssetConfig`](#0x7_confidential_asset_AssetConfig)
-  [Enum Resource `ConfidentialStore`](#0x7_confidential_asset_ConfidentialStore)
-  [Enum `Deposited`](#0x7_confidential_asset_Deposited)
-  [Enum `Withdrawn`](#0x7_confidential_asset_Withdrawn)
-  [Enum `Transferred`](#0x7_confidential_asset_Transferred)
-  [Enum `RegistrationProof`](#0x7_confidential_asset_RegistrationProof)
-  [Enum `WithdrawalProof`](#0x7_confidential_asset_WithdrawalProof)
-  [Enum `TransferProof`](#0x7_confidential_asset_TransferProof)
-  [Enum `NormalizationProof`](#0x7_confidential_asset_NormalizationProof)
-  [Enum `KeyRotationProof`](#0x7_confidential_asset_KeyRotationProof)
-  [Constants](#@Constants_0)
    -  [[test_only] The confidential asset module initialization failed.](#@[test_only]_The_confidential_asset_module_initialization_failed._1)
-  [Function `init_module`](#0x7_confidential_asset_init_module)
-  [Function `init_module_for_devnet`](#0x7_confidential_asset_init_module_for_devnet)
-  [Function `register_raw`](#0x7_confidential_asset_register_raw)
-  [Function `register`](#0x7_confidential_asset_register)
-  [Function `deposit`](#0x7_confidential_asset_deposit)
-  [Function `withdraw_raw`](#0x7_confidential_asset_withdraw_raw)
-  [Function `withdraw_to_raw`](#0x7_confidential_asset_withdraw_to_raw)
-  [Function `withdraw_to`](#0x7_confidential_asset_withdraw_to)
-  [Function `confidential_transfer_raw`](#0x7_confidential_asset_confidential_transfer_raw)
-  [Function `confidential_transfer`](#0x7_confidential_asset_confidential_transfer)
-  [Function `rotate_encryption_key_raw`](#0x7_confidential_asset_rotate_encryption_key_raw)
-  [Function `rotate_encryption_key`](#0x7_confidential_asset_rotate_encryption_key)
-  [Function `normalize_raw`](#0x7_confidential_asset_normalize_raw)
-  [Function `normalize`](#0x7_confidential_asset_normalize)
-  [Function `rollover_pending_balance`](#0x7_confidential_asset_rollover_pending_balance)
-  [Function `rollover_pending_balance_and_pause`](#0x7_confidential_asset_rollover_pending_balance_and_pause)
-  [Function `set_incoming_transfers_paused`](#0x7_confidential_asset_set_incoming_transfers_paused)
-  [Function `rollover_pending_balance_internal`](#0x7_confidential_asset_rollover_pending_balance_internal)
-  [Function `set_allow_listing`](#0x7_confidential_asset_set_allow_listing)
-  [Function `set_confidentiality_for_asset_type`](#0x7_confidential_asset_set_confidentiality_for_asset_type)
-  [Function `set_auditor_for_asset_type`](#0x7_confidential_asset_set_auditor_for_asset_type)
-  [Function `set_global_auditor`](#0x7_confidential_asset_set_global_auditor)
-  [Function `get_num_available_chunks`](#0x7_confidential_asset_get_num_available_chunks)
-  [Function `get_num_pending_chunks`](#0x7_confidential_asset_get_num_pending_chunks)
-  [Function `has_confidential_store`](#0x7_confidential_asset_has_confidential_store)
-  [Function `is_confidentiality_enabled_for_asset_type`](#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type)
-  [Function `is_allow_listing_enabled`](#0x7_confidential_asset_is_allow_listing_enabled)
-  [Function `get_pending_balance`](#0x7_confidential_asset_get_pending_balance)
-  [Function `get_available_balance`](#0x7_confidential_asset_get_available_balance)
-  [Function `get_encryption_key`](#0x7_confidential_asset_get_encryption_key)
-  [Function `is_normalized`](#0x7_confidential_asset_is_normalized)
-  [Function `incoming_transfers_paused`](#0x7_confidential_asset_incoming_transfers_paused)
-  [Function `get_auditor_for_asset_type`](#0x7_confidential_asset_get_auditor_for_asset_type)
-  [Function `get_global_auditor`](#0x7_confidential_asset_get_global_auditor)
-  [Function `get_effective_auditor`](#0x7_confidential_asset_get_effective_auditor)
-  [Function `get_global_auditor_epoch`](#0x7_confidential_asset_get_global_auditor_epoch)
-  [Function `get_auditor_epoch_for_asset_type`](#0x7_confidential_asset_get_auditor_epoch_for_asset_type)
-  [Function `get_effective_auditor_epoch`](#0x7_confidential_asset_get_effective_auditor_epoch)
-  [Function `get_total_confidential_supply`](#0x7_confidential_asset_get_total_confidential_supply)
-  [Function `get_num_transfers_received`](#0x7_confidential_asset_get_num_transfers_received)
-  [Function `get_max_transfers_before_rollover`](#0x7_confidential_asset_get_max_transfers_before_rollover)
-  [Function `get_asset_config_address`](#0x7_confidential_asset_get_asset_config_address)
-  [Function `get_asset_config_address_or_create`](#0x7_confidential_asset_get_asset_config_address_or_create)
-  [Function `get_global_config_signer`](#0x7_confidential_asset_get_global_config_signer)
-  [Function `get_global_config_address`](#0x7_confidential_asset_get_global_config_address)
-  [Function `get_confidential_store_signer`](#0x7_confidential_asset_get_confidential_store_signer)
-  [Function `get_confidential_store_address`](#0x7_confidential_asset_get_confidential_store_address)
-  [Function `borrow_confidential_store`](#0x7_confidential_asset_borrow_confidential_store)
-  [Function `borrow_confidential_store_mut`](#0x7_confidential_asset_borrow_confidential_store_mut)
-  [Function `get_asset_config_signer`](#0x7_confidential_asset_get_asset_config_signer)
-  [Function `construct_confidential_store_seed`](#0x7_confidential_asset_construct_confidential_store_seed)
-  [Function `construct_asset_config_seed`](#0x7_confidential_asset_construct_asset_config_seed)
-  [Function `assert_valid_registration_proof`](#0x7_confidential_asset_assert_valid_registration_proof)
-  [Function `assert_valid_withdrawal_proof`](#0x7_confidential_asset_assert_valid_withdrawal_proof)
-  [Function `assert_valid_transfer_proof`](#0x7_confidential_asset_assert_valid_transfer_proof)
-  [Function `assert_valid_normalization_proof`](#0x7_confidential_asset_assert_valid_normalization_proof)
-  [Function `assert_valid_key_rotation_proof`](#0x7_confidential_asset_assert_valid_key_rotation_proof)


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
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs">0x1::ristretto255_bulletproofs</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils">0x1::string_utils</a>;
<b>use</b> <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
<b>use</b> <a href="confidential_available_balance.md#0x7_confidential_available_balance">0x7::confidential_available_balance</a>;
<b>use</b> <a href="confidential_pending_balance.md#0x7_confidential_pending_balance">0x7::confidential_pending_balance</a>;
<b>use</b> <a href="confidential_proof.md#0x7_confidential_proof">0x7::confidential_proof</a>;
<b>use</b> <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation">0x7::sigma_protocol_key_rotation</a>;
<b>use</b> <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof">0x7::sigma_protocol_proof</a>;
<b>use</b> <a href="sigma_protocol_registration.md#0x7_sigma_protocol_registration">0x7::sigma_protocol_registration</a>;
<b>use</b> <a href="sigma_protocol_statement.md#0x7_sigma_protocol_statement">0x7::sigma_protocol_statement</a>;
<b>use</b> <a href="sigma_protocol_transfer.md#0x7_sigma_protocol_transfer">0x7::sigma_protocol_transfer</a>;
<b>use</b> <a href="sigma_protocol_utils.md#0x7_sigma_protocol_utils">0x7::sigma_protocol_utils</a>;
<b>use</b> <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw">0x7::sigma_protocol_withdraw</a>;
</code></pre>



<a id="0x7_confidential_asset_GlobalConfig"></a>

## Enum Resource `GlobalConfig`

A resource that represents the global configuration for the confidential asset protocol, "installed" during
<code>init_module</code> at @aptos_experimental.


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> <b>has</b> key
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
<code>global_auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>
 The global auditor's encryption key. If set, all confidential transfers must include the auditor
 as an additional party who can decrypt the transferred amount. Asset-specific auditors take
 precedence over this global auditor. If neither is set, no auditor is required.
</dd>
<dt>
<code>global_auditor_epoch: u64</code>
</dt>
<dd>
 Tracks how many times the global auditor EK has been installed or changed (not removed).
 Starts at 0 and increments each time a new EK is set (None→Some or Some(old)→Some(new)).
</dd>
<dt>
<code>extend_ref: <a href="../../aptos-framework/doc/object.md#0x1_object_ExtendRef">object::ExtendRef</a></code>
</dt>
<dd>
 Used to derive a signer that owns all the FAs' primary stores and <code><a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a></code> objects.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_AssetConfig"></a>

## Enum Resource `AssetConfig`

An object that represents the per-asset-type configuration.


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> <b>has</b> key
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
<code>auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>
 The auditor's public key for the asset type. If the auditor is not set, this field is <code>None</code>.
 Otherwise, each confidential transfer must include the auditor as an additional party,
 alongside the recipient, who has access to the decrypted transferred amount.

 TODO(Feature): add support for multiple auditors here
</dd>
<dt>
<code>auditor_epoch: u64</code>
</dt>
<dd>
 Tracks how many times the asset-specific auditor EK has been installed or changed (not removed).
 Starts at 0 and increments each time a new EK is set (None→Some or Some(old)→Some(new)).
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_ConfidentialStore"></a>

## Enum Resource `ConfidentialStore`

An object that stores the encrypted balances for a specific confidential asset type and owning user.
This should be thought of as a confidential variant of <code>aptos_framework::fungible_asset::FungibleStore</code>.

e.g., for Alice's confidential APT, such an object will be created and stored at an Alice-specific and APT-specific
address. It will track Alice's confidential APT balance.


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> <b>has</b> key
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
<code>pending_balance: <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a></code>
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
<code>available_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a></code>
</dt>
<dd>
 Represents the user's balance that is available for sending payments.
 It consists of eight 16-bit chunks $(a_0 + 2^{16} \cdot a_1 + ... + (2^{16})^15 \cdot a_15)$, supporting a
 128-bit balance. Includes A components for auditor decryption (empty if no auditor).
</dd>
<dt>
<code>ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a></code>
</dt>
<dd>
 The encryption key associated with the user's confidential asset account, different for each asset type.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_Deposited"></a>

## Enum `Deposited`

Emitted when someone brings confidential assets into the protocol via <code>deposit</code>: i.e., by depositing a fungible
asset into the "confidential pool" and minting a confidential asset as "proof" of this.


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x7_confidential_asset_Deposited">Deposited</a> <b>has</b> drop, store
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
<code>asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_Withdrawn"></a>

## Enum `Withdrawn`

Emitted when someone brings confidential assets out of the protocol via <code>withdraw_to</code>: i.e., by burning a confidential
asset as "proof" of being allowed to withdraw a fungible asset from the "confidential pool."


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x7_confidential_asset_Withdrawn">Withdrawn</a> <b>has</b> drop, store
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
<code>asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_Transferred"></a>

## Enum `Transferred`

Emitted when confidential assets are transferred within the protocol between users' confidential balances.
Note that a numeric amount is not included, as the whole point of the protocol is to avoid leaking it.


<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="confidential_asset.md#0x7_confidential_asset_Transferred">Transferred</a> <b>has</b> drop, store
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
<code>asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_RegistrationProof"></a>

## Enum `RegistrationProof`

Proof of knowledge of the decryption key for registration.
Contains a $\Sigma$-protocol proof that $H = \mathsf{dk} \cdot \mathsf{ek}$.


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_RegistrationProof">RegistrationProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_WithdrawalProof"></a>

## Enum `WithdrawalProof`

Represents the proof structure for validating a withdrawal operation.
Contains the sender's new normalized available balance, a range proof, and a
$\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{withdraw}$ relation.


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_WithdrawalProof">WithdrawalProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>new_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a></code>
</dt>
<dd>
 The sender's new normalized available balance, encrypted with fresh randomness.
</dd>
<dt>
<code>compressed_new_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a></code>
</dt>
<dd>
 The compressed form of <code>new_balance</code>, obtained at parse time to avoid recompression.
</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
</dd>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>
 $\Sigma$-protocol proof for the withdrawal relation.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_TransferProof"></a>

## Enum `TransferProof`

Represents the proof structure for validating a transfer operation.
Contains the sender's new balance, the transfer amount encrypted for the sender,
D-only components for the recipient and auditors, range proofs, and a
$\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{txfer}$ relation.


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_TransferProof">TransferProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>new_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a></code>
</dt>
<dd>
 The sender's new normalized available balance, encrypted with fresh randomness.
</dd>
<dt>
<code>compressed_new_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a></code>
</dt>
<dd>
 The compressed form of <code>new_balance</code>, obtained at parse time to avoid recompression.
</dd>
<dt>
<code>sender_amount: <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a></code>
</dt>
<dd>
 The transfer amount encrypted with the sender's encryption key.
</dd>
<dt>
<code>compressed_sender_amount: <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a></code>
</dt>
<dd>
 The compressed form of <code>sender_amount</code>, obtained at parse time to avoid recompression.
</dd>
<dt>
<code>recipient_amount_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>
 The D components of the transfer amount encrypted with the recipient's encryption key.
 The C components are the same as <code>sender_amount</code>'s C components (structurally guaranteed).
</dd>
<dt>
<code>compressed_recip_amount_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
</dt>
<dd>
 The compressed form of <code>recipient_R</code>, obtained at parse time to avoid recompression.
</dd>
<dt>
<code>auditor_amount_Ds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;&gt;</code>
</dt>
<dd>
 The D components of the transfer amount encrypted with each auditor's encryption key.
 The C components are the same as <code>sender_amount</code>'s C components (structurally guaranteed).
</dd>
<dt>
<code>compressed_auditor_Rs: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;&gt;</code>
</dt>
<dd>
 The compressed form of each auditor's D components, obtained at parse time to avoid recompression.
</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the resulting balance chunks for the sender are normalized.
</dd>
<dt>
<code>zkrp_amount: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the transferred amount chunks are normalized.
</dd>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>
 $\Sigma$-protocol proof for the transfer relation.
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_NormalizationProof"></a>

## Enum `NormalizationProof`

Represents the proof structure for validating a normalization operation.
Contains the user's new normalized available balance, a range proof, and a
$\Sigma$-protocol proof (reusing the withdrawal relation with $v = 0$).


<pre><code>enum <a href="confidential_asset.md#0x7_confidential_asset_NormalizationProof">NormalizationProof</a> <b>has</b> drop
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>new_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_AvailableBalance">confidential_available_balance::AvailableBalance</a></code>
</dt>
<dd>
 The user's new normalized available balance, encrypted with fresh randomness.
</dd>
<dt>
<code>compressed_new_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a></code>
</dt>
<dd>
 The compressed form of <code>new_balance</code>, obtained at parse time to avoid recompression.
</dd>
<dt>
<code>zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255_bulletproofs.md#0x1_ristretto255_bulletproofs_RangeProof">ristretto255_bulletproofs::RangeProof</a></code>
</dt>
<dd>
 Range proof ensuring that the resulting balance chunks are normalized (i.e., within the 16-bit limit).
</dd>
<dt>
<code>sigma: <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_Proof">sigma_protocol_proof::Proof</a></code>
</dt>
<dd>
 $\Sigma$-protocol proof for the normalization relation (withdrawal with $v = 0$).
</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_confidential_asset_KeyRotationProof"></a>

## Enum `KeyRotationProof`

Represents the proof structure for validating a key rotation operation.
Contains the new encryption key, the re-encrypted D components, and a $\Sigma$-protocol proof
that the re-encryption is correct.


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
<code>new_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>compressed_new_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;</code>
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



<a id="0x7_confidential_asset_E_AUDITOR_COUNT_MISMATCH"></a>

The number of auditor D-components in the proof does not match the expected auditor count.


<pre><code><b>const</b> <a href="confidential_asset.md#0x7_confidential_asset_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>: u64 = 12;
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
    // This is me being overly cautious: I added it <b>to</b> double-check my understanding that the VM always passes
    // the publishing <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> <b>as</b> deployer. It does, so the <b>assert</b> is redundant (it can never fail).
    <b>assert</b>!(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer) == @aptos_experimental, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INTERNAL_ERROR">E_INTERNAL_ERROR</a>));

    <b>assert</b>!(
        bulletproofs::get_max_range_bits() &gt;= <a href="confidential_proof.md#0x7_confidential_proof_get_bulletproofs_num_bits">confidential_proof::get_bulletproofs_num_bits</a>(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_internal">error::internal</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE">E_RANGE_PROOF_SYSTEM_HAS_INSUFFICIENT_RANGE</a>)
    );

    <b>let</b> deployer_address = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(deployer);
    <b>let</b> is_mainnet = <a href="../../aptos-framework/doc/chain_id.md#0x1_chain_id_get">chain_id::get</a>() == <a href="confidential_asset.md#0x7_confidential_asset_MAINNET_CHAIN_ID">MAINNET_CHAIN_ID</a>;

    <b>move_to</b>(
        deployer,
        GlobalConfig::V1 {
            allow_list_enabled: is_mainnet,
            global_auditor_ek: std::option::none(),
            global_auditor_epoch: 0,
            // DO NOT CHANGE: using long syntax until framework change is released <b>to</b> mainnet
            extend_ref: <a href="../../aptos-framework/doc/object.md#0x1_object_generate_extend_ref">object::generate_extend_ref</a>(&<a href="../../aptos-framework/doc/object.md#0x1_object_create_object">object::create_object</a>(deployer_address))
        }
    );

    // On mainnet, allow APT by default
    <b>if</b> (is_mainnet) {
        <b>let</b> apt_metadata = <a href="../../aptos-framework/doc/object.md#0x1_object_address_to_object">object::address_to_object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;(@aptos_fungible_asset);
        <b>let</b> config_signer = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_signer">get_asset_config_signer</a>(apt_metadata);
        <b>move_to</b>(&config_signer, AssetConfig::V1 { allowed: <b>true</b>, auditor_ek: std::option::none(), auditor_epoch: 0 });
    };
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

<a id="0x7_confidential_asset_register_raw"></a>

## Function `register_raw`

Registers an account for a specified asset type.
Parses arguments and forwards to <code>register</code>; see that function for details.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register_raw">register_raw</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register_raw">register_raw</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>let</b> ek = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(ek).extract();
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(sigma_proto_comm, sigma_proto_resp);
    <b>let</b> proof = RegistrationProof::V1 { sigma };

    <a href="confidential_asset.md#0x7_confidential_asset_register">register</a>(sender, asset_type, ek, proof);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_register"></a>

## Function `register`

Registers an a confidential store for a specified asset type, encrypted under the given EK.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register">register</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, proof: <a href="confidential_asset.md#0x7_confidential_asset_RegistrationProof">confidential_asset::RegistrationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_register">register</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type:
    Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: CompressedRistretto,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_RegistrationProof">RegistrationProof</a>
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));

    <b>assert</b>!(
        !<a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender), asset_type),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_ALREADY_REGISTERED">E_CONFIDENTIAL_STORE_ALREADY_REGISTERED</a>)
    );

    // Makes sure the user knows their <a href="../../aptos-framework/doc/decryption.md#0x1_decryption">decryption</a> key.
    <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_registration_proof">assert_valid_registration_proof</a>(sender, asset_type, &ek, proof);

    <b>let</b> ca_store = ConfidentialStore::V1 {
        pause_incoming: <b>false</b>,
        normalized: <b>true</b>,
        transfers_received: 0,
        pending_balance: <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_zero_compressed">confidential_pending_balance::new_zero_compressed</a>(),
        available_balance: <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_zero_compressed">confidential_available_balance::new_zero_compressed</a>(),
        ek
    };

    <b>move_to</b>(&<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_signer">get_confidential_store_signer</a>(sender, asset_type), ca_store);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_deposit"></a>

## Function `deposit`

Brings tokens into the protocol, transferring the passed amount from the sender's primary FA store
to the sender's own pending balance.
The initial confidential balance is publicly visible, as entering the protocol requires a normal transfer.
However, tokens within the protocol become obfuscated through confidential transfers, ensuring privacy in
subsequent transactions.

For convenience, we sometimes refer to this operation as "veiling."


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit">deposit</a>(depositor: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_deposit">deposit</a>(
    depositor: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    amount: u64
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>let</b> addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(depositor);

    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));
    <b>assert</b>!(!<a href="confidential_asset.md#0x7_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(addr, asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>));

    // Note: This sets up the "confidential asset pool" for this asset type, <b>if</b> one is not already set up, such <b>as</b>
    // when someone first veils this asset type for the first time.
    <b>let</b> pool_fa_store = <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_ensure_primary_store_exists">primary_fungible_store::ensure_primary_store_exists</a>(
        <a href="confidential_asset.md#0x7_confidential_asset_get_global_config_address">get_global_config_address</a>(), asset_type
    );

    // Step 1: Transfer the asset from the user's <a href="../../aptos-framework/doc/account.md#0x1_account">account</a> into the confidential asset pool
    <b>let</b> depositor_fa_store = <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_primary_store">primary_fungible_store::primary_store</a>(addr, asset_type);
    <a href="../../aptos-framework/doc/dispatchable_fungible_asset.md#0x1_dispatchable_fungible_asset_transfer">dispatchable_fungible_asset::transfer</a>(depositor, depositor_fa_store, pool_fa_store, amount);

    // Step 2: "Mint" corresponding confidential assets for the depositor, and add them <b>to</b> their pending balance.
    <b>let</b> ca_store = <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(addr, asset_type);

    // Make sure the depositor <b>has</b> "room" in their pending balance for this deposit
    <b>assert</b>!(
        ca_store.transfers_received &lt; <a href="confidential_asset.md#0x7_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>)
    );

    ca_store.pending_balance.add_assign(&<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_u64_no_randomness">confidential_pending_balance::new_u64_no_randomness</a>(amount));
    ca_store.transfers_received += 1;

    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(Deposited::V1 { addr, amount, asset_type });
}
</code></pre>



</details>

<a id="0x7_confidential_asset_withdraw_raw"></a>

## Function `withdraw_raw`

The same as <code>withdraw_to_raw</code>, but the recipient is the sender.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_raw">withdraw_raw</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64, new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_raw">withdraw_raw</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, amount: u64,
    new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to_raw">withdraw_to_raw</a>(
        sender,
        asset_type,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender),
        amount,
        new_balance_C,
        new_balance_D,
        new_balance_A,
        zkrp_new_balance,
        sigma_proto_comm,
        sigma_proto_resp
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_withdraw_to_raw"></a>

## Function `withdraw_to_raw`

Brings tokens out of the protocol by transferring the specified amount from the sender's available balance to
the recipient's primary FA store.
Parses arguments and forwards to <code>withdraw_to</code>; see that function for details.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to_raw">withdraw_to_raw</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64, new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to_raw">withdraw_to_raw</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64,
    new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>let</b> (new_P, compressed_P) = deserialize_points(new_balance_C);
    <b>let</b> (new_R, compressed_R) = deserialize_points(new_balance_D);
    <b>let</b> (new_R_aud, compressed_R_aud) = deserialize_points(new_balance_A);
    <b>let</b> new_balance = <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_p_r_r_aud">confidential_available_balance::new_from_p_r_r_aud</a>(new_P, new_R, new_R_aud);
    <b>let</b> compressed_new_balance = <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">confidential_available_balance::new_compressed_from_p_r_r_aud</a>(
        compressed_P, compressed_R, compressed_R_aud
    );
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(sigma_proto_comm, sigma_proto_resp);
    <b>let</b> proof = WithdrawalProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma };

    <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to">withdraw_to</a>(sender, asset_type, <b>to</b>, amount, proof);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_withdraw_to"></a>

## Function `withdraw_to`

Brings tokens out of the protocol by transferring the specified amount from the sender's available balance to
the recipient's primary FA store.
The withdrawn amount is publicly visible, as this process requires a normal transfer.
The proof contains the sender's new normalized confidential balance, encrypted with fresh randomness.
Withdrawals are always allowed, regardless of whether the asset type is allow-listed.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to">withdraw_to</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, amount: u64, proof: <a href="confidential_asset.md#0x7_confidential_asset_WithdrawalProof">confidential_asset::WithdrawalProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_withdraw_to">withdraw_to</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    amount: u64,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_WithdrawalProof">WithdrawalProof</a>
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>let</b> sender_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    // Read values before mutable borrow <b>to</b> avoid conflicting borrows of <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>
    <b>let</b> ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(sender_addr, asset_type);
    <b>let</b> current_balance = <a href="confidential_asset.md#0x7_confidential_asset_get_available_balance">get_available_balance</a>(sender_addr, asset_type);
    <b>let</b> auditor_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_effective_auditor">get_effective_auditor</a>(asset_type);

    <b>let</b> compressed_new_balance = <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_withdrawal_proof">assert_valid_withdrawal_proof</a>(
        sender,
        asset_type,
        &ek,
        amount,
        &current_balance,
        &auditor_ek,
        proof
    );

    <b>let</b> ca_store = <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(sender_addr, asset_type);
    ca_store.normalized = <b>true</b>;
    ca_store.available_balance = compressed_new_balance;

    <a href="../../aptos-framework/doc/primary_fungible_store.md#0x1_primary_fungible_store_transfer">primary_fungible_store::transfer</a>(&<a href="confidential_asset.md#0x7_confidential_asset_get_global_config_signer">get_global_config_signer</a>(), asset_type, <b>to</b>, amount);

    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(Withdrawn::V1 { from: sender_addr, <b>to</b>, amount, asset_type });
}
</code></pre>



</details>

<a id="0x7_confidential_asset_confidential_transfer_raw"></a>

## Function `confidential_transfer_raw`

Transfers tokens from the sender's available balance to the recipient's pending balance.
Parses arguments and forwards to <code>confidential_transfer</code>; see that function for details.

The <code>extra_auditor_eks</code> should contain only additional auditor EKs (not the global auditor,
which is fetched automatically by the contract).

Only the D components are sent for the recipient and auditors, since they share the same
C components as the sender's amount (C_i = amount_i * G + r_i * H). This saves 128 bytes
per party (recipient + each auditor).


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer_raw">confidential_transfer_raw</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sender_amount_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sender_amount_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, recipient_amount_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, extra_auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, auditor_amount_Ds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, zkrp_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer_raw">confidential_transfer_raw</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sender_amount_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sender_amount_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    recipient_amount_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    extra_auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    auditor_amount_Ds: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    zkrp_amount: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    // Deserialize all point components, obtaining both decompressed and compressed forms in one pass
    <b>let</b> (new_P, compressed_P) = deserialize_points(new_balance_C);
    <b>let</b> (new_R, compressed_R) = deserialize_points(new_balance_D);
    <b>let</b> (new_R_aud, compressed_R_aud) = deserialize_points(new_balance_A);
    <b>let</b> new_balance = <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_p_r_r_aud">confidential_available_balance::new_from_p_r_r_aud</a>(new_P, new_R, new_R_aud);
    <b>let</b> compressed_new_balance = <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">confidential_available_balance::new_compressed_from_p_r_r_aud</a>(
        compressed_P, compressed_R, compressed_R_aud
    );

    <b>let</b> (sender_P, compressed_sender_P) = deserialize_points(sender_amount_C);
    <b>let</b> (sender_R, compressed_sender_R) = deserialize_points(sender_amount_D);
    <b>let</b> sender_amount = <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">confidential_pending_balance::new_from_p_and_r</a>(sender_P, sender_R);
    <b>let</b> compressed_sender_amount = <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_compressed_from_p_and_r">confidential_pending_balance::new_compressed_from_p_and_r</a>(
        compressed_sender_P, compressed_sender_R
    );

    <b>let</b> (recipient_amount_R, compressed_recipient_amount_R) = deserialize_points(recipient_amount_R);

    <b>let</b> extra_auditor_eks = extra_auditor_eks.map(|bytes| {
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(bytes).extract()
    });

    <b>let</b> decompressed_auditor_amount_Ds = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> compressed_auditor_Rs = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    auditor_amount_Ds.for_each(|auditor_d| {
        <b>let</b> (d, cd) = deserialize_points(auditor_d);
        decompressed_auditor_amount_Ds.push_back(d);
        compressed_auditor_Rs.push_back(cd);
    });

    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
    <b>let</b> zkrp_transfer_amount = bulletproofs::range_proof_from_bytes(zkrp_amount);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(sigma_proto_comm, sigma_proto_resp);
    <b>let</b> proof = TransferProof::V1 {
        new_balance, compressed_new_balance,
        sender_amount, compressed_sender_amount,
        recipient_amount_R, compressed_recip_amount_R: compressed_recipient_amount_R,
        auditor_amount_Ds: decompressed_auditor_amount_Ds, compressed_auditor_Rs,
        zkrp_new_balance, zkrp_amount: zkrp_transfer_amount, sigma
    };

    <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer">confidential_transfer</a>(
        sender,
        asset_type,
        <b>to</b>,
        extra_auditor_eks,
        proof
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_confidential_transfer"></a>

## Function `confidential_transfer`

Transfers tokens from the sender's available balance to the recipient's pending balance.
The function hides the transferred amount while keeping the sender and recipient addresses visible.
The proof contains: the sender's new balance, the transfer amount encrypted for sender/recipient/auditors,
and range proofs for the new balance and transfer amount.
The <code>extra_auditor_eks</code> should contain any additional auditor EKs beyond the global auditor
(which is fetched automatically by the contract).


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer">confidential_transfer</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, <b>to</b>: <b>address</b>, extra_auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, proof: <a href="confidential_asset.md#0x7_confidential_asset_TransferProof">confidential_asset::TransferProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_confidential_transfer">confidential_transfer</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    <b>to</b>: <b>address</b>,
    extra_auditor_eks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_TransferProof">TransferProof</a>
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ASSET_TYPE_DISALLOWED">E_ASSET_TYPE_DISALLOWED</a>));
    <b>assert</b>!(!<a href="confidential_asset.md#0x7_confidential_asset_incoming_transfers_paused">incoming_transfers_paused</a>(<b>to</b>, asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_PAUSED">E_INCOMING_TRANSFERS_PAUSED</a>));

    <b>let</b> from = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    // Compute extra auditor count before appending effective auditor
    <b>let</b> num_extra_auditors = extra_auditor_eks.length();

    // Append effective auditor EK (asset-specific first, then <b>global</b> fallback) <b>to</b> extra_auditor_eks
    <b>let</b> effective_auditor_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_effective_auditor">get_effective_auditor</a>(asset_type);
    <b>let</b> has_effective_auditor = effective_auditor_ek.is_some();
    <b>if</b> (has_effective_auditor) {
        extra_auditor_eks.push_back(effective_auditor_ek.extract());
    };

    // Read values before mutable borrow <b>to</b> avoid conflicting borrows of <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>
    <b>let</b> sender_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(from, asset_type);
    <b>let</b> recipient_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(<b>to</b>, asset_type);
    <b>let</b> sender_available_balance = <a href="confidential_asset.md#0x7_confidential_asset_get_available_balance">get_available_balance</a>(from, asset_type);

    // Note: Sender's amount is not used: we pass it <b>as</b> an argument just for visibility, so that indexing can reliably
    // pick it up for dapps that need <b>to</b> decrypt it quickly.
    <b>let</b> (compressed_new_balance, _sender_amount, recipient_amount, _auditor_amounts) =
        <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_transfer_proof">assert_valid_transfer_proof</a>(
            sender,
            <b>to</b>,
            asset_type,
            &sender_ek,
            &recipient_ek,
            &sender_available_balance,
            &extra_auditor_eks,
            has_effective_auditor,
            num_extra_auditors,
            proof
        );

    // Update sender's confidential store
    <b>let</b> sender_ca_store = <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(from, asset_type);
    sender_ca_store.normalized = <b>true</b>;
    sender_ca_store.available_balance = compressed_new_balance;

    // Update recipient's confidential store
    <b>let</b> recip_ca_store = <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(<b>to</b>, asset_type);
    // Make sure the receiver <b>has</b> "room" in their pending balance for this transfer
    <b>assert</b>!(
        recip_ca_store.transfers_received &lt; <a href="confidential_asset.md#0x7_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_MUST_BE_ROLLED_OVER">E_PENDING_BALANCE_MUST_BE_ROLLED_OVER</a>)
    );
    recip_ca_store.pending_balance.add_assign(&recipient_amount);
    recip_ca_store.transfers_received += 1;

    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(Transferred::V1 { from, <b>to</b>, asset_type });
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rotate_encryption_key_raw"></a>

## Function `rotate_encryption_key_raw`

Rotates the encryption key for the user's confidential balance, updating it to a new encryption key.
Parses arguments and forwards to <code>rotate_encryption_key</code>; see that function for details.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key_raw">rotate_encryption_key_raw</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, resume_incoming_transfers: bool, new_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key_raw">rotate_encryption_key_raw</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    resume_incoming_transfers: bool,
    new_R: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
    sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
    sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, // part of the proof
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    // Just parse stuff and forward <b>to</b> the more type-safe function
    <b>let</b> (new_ek, compressed_new_ek) = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_point_and_compressed_from_bytes">ristretto255::new_point_and_compressed_from_bytes</a>(new_ek);
    <b>let</b> (new_R, compressed_new_R) = deserialize_points(new_R);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(
        sigma_proto_comm, sigma_proto_resp
    );

    <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(
        sender, asset_type, new_ek,
        KeyRotationProof::V1 { compressed_new_ek, new_R, compressed_new_R, sigma },
        resume_incoming_transfers
    );
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rotate_encryption_key"></a>

## Function `rotate_encryption_key`



<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, proof: <a href="confidential_asset.md#0x7_confidential_asset_KeyRotationProof">confidential_asset::KeyRotationProof</a>, resume_incoming_transfers: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rotate_encryption_key">rotate_encryption_key</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_ek: RistrettoPoint,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_KeyRotationProof">KeyRotationProof</a>,
    resume_incoming_transfers: bool,
) {
    // Step 1: Assert (a) incoming transfers are paused & (b) pending balance is zero / <b>has</b> been rolled over
    <b>let</b> ca_store = <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type);
    // (a) Assert incoming transfers are paused & unpause them after, <b>if</b> flag is set.
    <b>assert</b>!(ca_store.pause_incoming, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_INCOMING_TRANSFERS_NOT_PAUSED">E_INCOMING_TRANSFERS_NOT_PAUSED</a>));
    // (b) The user must have called `rollover_pending_balance` before rotating their key.
    <b>assert</b>!(
        ca_store.pending_balance.is_zero(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>)
    );
    // Over-asserting invariants, in an abundance of caution.
    <b>assert</b>!(
        ca_store.transfers_received == 0,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION">E_PENDING_BALANCE_NOT_ZERO_BEFORE_KEY_ROTATION</a>)
    );

    // Step 2: Verify the $\Sigma$-protocol proof of correct re-encryption
    <b>let</b> (compressed_new_ek, compressed_new_R) = <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_key_rotation_proof">assert_valid_key_rotation_proof</a>(
        owner, asset_type, new_ek, &ca_store.ek, &ca_store.available_balance, proof
    );

    // Step 3: Install the new EK and the new re-encrypted available balance
    ca_store.ek = compressed_new_ek;
    // We're just updating the available balance's EK-dependant D-component & leaving the pending balance the same.
    ca_store.available_balance.set_compressed_R(compressed_new_R);
    <b>if</b> (resume_incoming_transfers) {
        ca_store.pause_incoming = <b>false</b>;
    }
}
</code></pre>



</details>

<a id="0x7_confidential_asset_normalize_raw"></a>

## Function `normalize_raw`

Adjusts each chunk to fit into defined 16-bit bounds to prevent overflows.
Parses arguments and forwards to <code>normalize</code>; see that function for details.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize_raw">normalize_raw</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize_raw">normalize_raw</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_balance_C: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_D: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    new_balance_A: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    zkrp_new_balance: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    sigma_proto_comm: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    sigma_proto_resp: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> (new_P, compressed_P) = deserialize_points(new_balance_C);
    <b>let</b> (new_R, compressed_R) = deserialize_points(new_balance_D);
    <b>let</b> (new_R_aud, compressed_R_aud) = deserialize_points(new_balance_A);
    <b>let</b> new_balance = <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_from_p_r_r_aud">confidential_available_balance::new_from_p_r_r_aud</a>(new_P, new_R, new_R_aud);
    <b>let</b> compressed_new_balance = <a href="confidential_available_balance.md#0x7_confidential_available_balance_new_compressed_from_p_r_r_aud">confidential_available_balance::new_compressed_from_p_r_r_aud</a>(
        compressed_P, compressed_R, compressed_R_aud
    );
    <b>let</b> zkrp_new_balance = bulletproofs::range_proof_from_bytes(zkrp_new_balance);
    <b>let</b> sigma = <a href="sigma_protocol_proof.md#0x7_sigma_protocol_proof_new_proof_from_bytes">sigma_protocol_proof::new_proof_from_bytes</a>(sigma_proto_comm, sigma_proto_resp);
    <b>let</b> proof = NormalizationProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma };

    <a href="confidential_asset.md#0x7_confidential_asset_normalize">normalize</a>(sender, asset_type, proof);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_normalize"></a>

## Function `normalize`

Adjusts each chunk to fit into defined 16-bit bounds to prevent overflows.
Most functions perform implicit normalization by accepting a new normalized confidential balance as a parameter.
However, explicit normalization is required before rolling over the pending balance, as multiple rolls may cause
chunk overflows.
The proof contains the sender's new normalized confidential balance, encrypted with fresh randomness.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize">normalize</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, proof: <a href="confidential_asset.md#0x7_confidential_asset_NormalizationProof">confidential_asset::NormalizationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_normalize">normalize</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_NormalizationProof">NormalizationProof</a>
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> user = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);

    // Check normalized flag and read values before mutable borrow
    <b>assert</b>!(!<a href="confidential_asset.md#0x7_confidential_asset_is_normalized">is_normalized</a>(user, asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_ALREADY_NORMALIZED">E_ALREADY_NORMALIZED</a>));
    <b>let</b> ek = <a href="confidential_asset.md#0x7_confidential_asset_get_encryption_key">get_encryption_key</a>(user, asset_type);
    <b>let</b> current_balance = <a href="confidential_asset.md#0x7_confidential_asset_get_available_balance">get_available_balance</a>(user, asset_type);
    <b>let</b> auditor_ek = <a href="confidential_asset.md#0x7_confidential_asset_get_effective_auditor">get_effective_auditor</a>(asset_type);

    <b>let</b> compressed_new_balance = <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_normalization_proof">assert_valid_normalization_proof</a>(
        sender,
        asset_type,
        &ek,
        &current_balance,
        &auditor_ek,
        proof
    );

    <b>let</b> ca_store = <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(user, asset_type);
    ca_store.available_balance = compressed_new_balance;
    ca_store.normalized = <b>true</b>;
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
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_internal">rollover_pending_balance_internal</a>(sender, asset_type);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_rollover_pending_balance_and_pause"></a>

## Function `rollover_pending_balance_and_pause`

Before calling <code>rotate_encryption_key_raw</code>, we need to rollover the pending balance and pause incoming transfers
for this asset type to prevent any new transfers from coming in.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_and_pause">rollover_pending_balance_and_pause</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance_and_pause">rollover_pending_balance_and_pause</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_rollover_pending_balance">rollover_pending_balance</a>(sender, asset_type);
    <a href="confidential_asset.md#0x7_confidential_asset_set_incoming_transfers_paused">set_incoming_transfers_paused</a>(sender, asset_type, <b>true</b>);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_set_incoming_transfers_paused"></a>

## Function `set_incoming_transfers_paused`

Pauses or resumes incoming transfers for the specified account and asset type.
Pausing is needed before rotating the encryption key: the owner must pause incoming transfers so as to be able
to roll over their pending balance fully. Then, to rotate their encryption key, the owner needs to only re-encrypt
their available balance ciphertext. Once done, the owner can unpause incoming transfers.


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_incoming_transfers_paused">set_incoming_transfers_paused</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, paused: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_incoming_transfers_paused">set_incoming_transfers_paused</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    paused: bool
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner), asset_type).pause_incoming = paused;
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
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>let</b> user = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(sender);
    <b>let</b> ca_store = <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(user, asset_type);

    <b>assert</b>!(ca_store.normalized, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_NORMALIZATION_REQUIRED">E_NORMALIZATION_REQUIRED</a>));

    ca_store.available_balance.add_assign(&ca_store.pending_balance);
    // A components remain stale — will be refreshed on normalize/withdraw/transfer

    ca_store.normalized = <b>false</b>;
    ca_store.transfers_received = 0;
    ca_store.pending_balance = <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_zero_compressed">confidential_pending_balance::new_zero_compressed</a>();
}
</code></pre>



</details>

<a id="0x7_confidential_asset_set_allow_listing"></a>

## Function `set_allow_listing`

Enables or disables the allow list. When enabled, only asset types from the allow list can be transferred.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_allow_listing">set_allow_listing</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enabled: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_allow_listing">set_allow_listing</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, enabled: bool) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).allow_list_enabled = enabled;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_set_confidentiality_for_asset_type"></a>

## Function `set_confidentiality_for_asset_type`

Enables or disables confidential transfers for the specified asset type.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_confidentiality_for_asset_type">set_confidentiality_for_asset_type</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, allowed: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_confidentiality_for_asset_type">set_confidentiality_for_asset_type</a>(
    aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    allowed: bool
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> asset_config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type));
    asset_config.allowed = allowed;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_set_auditor_for_asset_type"></a>

## Function `set_auditor_for_asset_type`

Sets or removes the auditor for the specified asset type.

Notes:
- Ensures that new_auditor_ek is a valid Ristretto255 point
- Ideally, this should require a ZKPoK of DK too. But, instead, we assume competent auditors.

The <code>auditor_epoch</code> is incremented only when installing or changing the EK (not when removing):
- None → Some(ek): epoch increments (installing)
- Some(old) → Some(new) where old != new: epoch increments (changing)
- Some(old) → Some(old): epoch stays (no change)
- Some(_) → None: epoch stays (removing)
- None → None: epoch stays (no-op)


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_auditor_for_asset_type">set_auditor_for_asset_type</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_auditor_for_asset_type">set_auditor_for_asset_type</a>(
    aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    auditor_ek: Option&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> asset_config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type));

    <b>let</b> new_ek = auditor_ek.map(|ek|
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(ek).extract()
    );

    // Increment epoch only when installing or changing the EK (not when removing)
    <b>let</b> should_increment = <b>if</b> (new_ek.is_some()) {
        <b>if</b> (asset_config.auditor_ek.is_some()) {
            !new_ek.borrow().compressed_point_equals(asset_config.auditor_ek.borrow())
        } <b>else</b> {
            <b>true</b> // None → Some: installing
        }
    } <b>else</b> {
        <b>false</b> // removing or no-op
    };

    <b>if</b> (should_increment) {
        asset_config.auditor_epoch = asset_config.auditor_epoch + 1;
    };

    asset_config.auditor_ek = new_ek;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_set_global_auditor"></a>

## Function `set_global_auditor`

Sets or removes the global auditor for all asset types. The global auditor is used as a fallback when no
asset-specific auditor is set. (Ideally, this should require a ZKPoK of DK but we assume competent auditors.)

The <code>global_auditor_epoch</code> is incremented only when installing or changing the EK (not when removing):
- None → Some(ek): epoch increments (installing)
- Some(old) → Some(new) where old != new: epoch increments (changing)
- Some(old) → Some(old): epoch stays (no change)
- Some(_) → None: epoch stays (removing)
- None → None: epoch stays (no-op)


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_global_auditor">set_global_auditor</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, auditor_ek: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_set_global_auditor">set_global_auditor</a>(aptos_framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, auditor_ek: Option&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental);

    <b>let</b> new_ek = auditor_ek.map(|ek|
        <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_compressed_point_from_bytes">ristretto255::new_compressed_point_from_bytes</a>(ek).extract()
    );

    // Increment epoch only when installing or changing the EK (not when removing)
    <b>let</b> should_increment = <b>if</b> (new_ek.is_some()) {
        <b>if</b> (config.global_auditor_ek.is_some()) {
            !new_ek.borrow().compressed_point_equals(config.global_auditor_ek.borrow())
        } <b>else</b> {
            <b>true</b> // None → Some: installing
        }
    } <b>else</b> {
        <b>false</b> // removing or no-op
    };

    <b>if</b> (should_increment) {
        config.global_auditor_epoch = config.global_auditor_epoch + 1;
    };

    config.global_auditor_ek = new_ek;
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_num_available_chunks"></a>

## Function `get_num_available_chunks`

Helper to get the number of available balance chunks.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_num_available_chunks">get_num_available_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_num_available_chunks">get_num_available_chunks</a>(): u64 {
    <a href="confidential_available_balance.md#0x7_confidential_available_balance_get_num_chunks">confidential_available_balance::get_num_chunks</a>()
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_num_pending_chunks"></a>

## Function `get_num_pending_chunks`

Helper to get the number of pending balance chunks.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_num_pending_chunks">get_num_pending_chunks</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_num_pending_chunks">get_num_pending_chunks</a>(): u64 {
    <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_get_num_chunks">confidential_pending_balance::get_num_chunks</a>()
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


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_confidentiality_enabled_for_asset_type">is_confidentiality_enabled_for_asset_type</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): bool <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a> {
    <b>if</b> (!<a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>()) {
        <b>return</b> <b>true</b>
    };

    <b>let</b> asset_config_address = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);

    <b>if</b> (!<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address)) {
        <b>return</b> <b>false</b>
    };

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address).allowed
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


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>(): bool <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).allow_list_enabled
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_pending_balance"></a>

## Function `get_pending_balance`

Returns the pending balance of the user for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_pending_balance">get_pending_balance</a>(owner: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_CompressedPendingBalance">confidential_pending_balance::CompressedPendingBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_pending_balance">get_pending_balance</a>(
    owner: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): CompressedPendingBalance <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(owner, asset_type).pending_balance
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_available_balance"></a>

## Function `get_available_balance`

Returns the available balance of the user for the specified asset type.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_available_balance">get_available_balance</a>(owner: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_available_balance">get_available_balance</a>(
    owner: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): CompressedAvailableBalance <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(owner, asset_type).available_balance
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
    <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).ek
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
    <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).normalized
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
    <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).pause_incoming
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
): Option&lt;CompressedRistretto&gt; <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> asset_config_address = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);

    <b>if</b> (!<a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>() && !<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address)) {
        <b>return</b> std::option::none();
    };

    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address).auditor_ek
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_global_auditor"></a>

## Function `get_global_auditor`

Returns the global auditor's encryption key, or <code>None</code> if no global auditor is set.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_auditor">get_global_auditor</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_auditor">get_global_auditor</a>(): Option&lt;CompressedRistretto&gt; <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).global_auditor_ek
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_effective_auditor"></a>

## Function `get_effective_auditor`

Returns the effective auditor for a given asset type, checking the asset-specific auditor first
and falling back to the global auditor.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_effective_auditor">get_effective_auditor</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_effective_auditor">get_effective_auditor</a>(
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): Option&lt;CompressedRistretto&gt; <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    // 1. Check asset-specific auditor
    <b>let</b> config_addr = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);
    <b>if</b> (<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr)) {
        <b>let</b> asset_auditor = <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr).auditor_ek;
        <b>if</b> (asset_auditor.is_some()) {
            <b>return</b> asset_auditor
        };
    };
    // 2. Fall back <b>to</b> <b>global</b> auditor
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).global_auditor_ek
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_global_auditor_epoch"></a>

## Function `get_global_auditor_epoch`

Returns the global auditor epoch counter.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_auditor_epoch">get_global_auditor_epoch</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_auditor_epoch">get_global_auditor_epoch</a>(): u64 <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).global_auditor_epoch
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_auditor_epoch_for_asset_type"></a>

## Function `get_auditor_epoch_for_asset_type`

Returns the auditor epoch counter for a specific asset type. Returns 0 if no <code><a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a></code>
exists for this asset type (and allow-listing is disabled).


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_auditor_epoch_for_asset_type">get_auditor_epoch_for_asset_type</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_auditor_epoch_for_asset_type">get_auditor_epoch_for_asset_type</a>(
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): u64 <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> asset_config_address = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);
    <b>if</b> (!<a href="confidential_asset.md#0x7_confidential_asset_is_allow_listing_enabled">is_allow_listing_enabled</a>() && !<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address)) {
        <b>return</b> 0
    };
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(asset_config_address).auditor_epoch
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_effective_auditor_epoch"></a>

## Function `get_effective_auditor_epoch`

Returns the effective auditor epoch: asset-specific epoch if the asset has an auditor,
otherwise global auditor epoch.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_effective_auditor_epoch">get_effective_auditor_epoch</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_effective_auditor_epoch">get_effective_auditor_epoch</a>(
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;
): u64 <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>, <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> config_addr = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);
    <b>if</b> (<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr)) {
        <b>let</b> ac = <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(config_addr);
        <b>if</b> (ac.auditor_ek.is_some()) {
            <b>return</b> ac.auditor_epoch
        };
    };
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).global_auditor_epoch
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_total_confidential_supply"></a>

## Function `get_total_confidential_supply`

Returns the circulating supply of the confidential asset.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_total_confidential_supply">get_total_confidential_supply</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_total_confidential_supply">get_total_confidential_supply</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): u64 <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> fa_store_address = <a href="confidential_asset.md#0x7_confidential_asset_get_global_config_address">get_global_config_address</a>();
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
    <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user, asset_type).transfers_received
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_max_transfers_before_rollover"></a>

## Function `get_max_transfers_before_rollover`

Returns the maximum number of transfers that can be accumulated in the pending balance before rollover is required.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_max_transfers_before_rollover">get_max_transfers_before_rollover</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_max_transfers_before_rollover">get_max_transfers_before_rollover</a>(): u64 {
    <a href="confidential_asset.md#0x7_confidential_asset_MAX_TRANSFERS_BEFORE_ROLLOVER">MAX_TRANSFERS_BEFORE_ROLLOVER</a>
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_asset_config_address"></a>

## Function `get_asset_config_address`

Returns the address that handles primary FA store and <code><a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a></code> objects for the specified asset type.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> config_ext = &<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).extend_ref;
    <b>let</b> config_ext_address = <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(config_ext);
    <a href="../../aptos-framework/doc/object.md#0x1_object_create_object_address">object::create_object_address</a>(&config_ext_address, <a href="confidential_asset.md#0x7_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_asset_config_address_or_create"></a>

## Function `get_asset_config_address_or_create`

Ensures that the <code><a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a></code> object exists for the specified asset type and returns its address.
If the object does not exist, creates it. Used only for internal purposes.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address_or_create">get_asset_config_address_or_create</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> addr = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_address">get_asset_config_address</a>(asset_type);

    <b>if</b> (!<b>exists</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>&gt;(addr)) {
        <b>let</b> asset_config_signer = <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_signer">get_asset_config_signer</a>(asset_type);

        <b>move_to</b>(
            &asset_config_signer,
            // We disallow the asset type from being made confidential since this function is
            // called in a lot of different contexts.
            AssetConfig::V1 { allowed: <b>false</b>, auditor_ek: std::option::none(), auditor_epoch: 0 }
        );
    };

    addr
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_global_config_signer"></a>

## Function `get_global_config_signer`

Returns an object for handling all the FA primary stores, and returns a signer for it.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_config_signer">get_global_config_signer</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_config_signer">get_global_config_signer</a>(): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(&<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).extend_ref)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_global_config_address"></a>

## Function `get_global_config_address`

Returns the address that handles all the FA primary stores.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_config_address">get_global_config_address</a>(): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_global_config_address">get_global_config_address</a>(): <b>address</b> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <a href="../../aptos-framework/doc/object.md#0x1_object_address_from_extend_ref">object::address_from_extend_ref</a>(&<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).extend_ref)
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

<a id="0x7_confidential_asset_borrow_confidential_store"></a>

## Function `borrow_confidential_store`

Borrows the <code><a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a></code> for the given user and asset type, asserting it exists.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">confidential_asset::ConfidentialStore</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store">borrow_confidential_store</a>(user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>));
    <b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_borrow_confidential_store_mut"></a>

## Function `borrow_confidential_store_mut`

Mutably borrows the <code><a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a></code> for the given user and asset type, asserting it exists.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(user: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<b>mut</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">confidential_asset::ConfidentialStore</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_borrow_confidential_store_mut">borrow_confidential_store_mut</a>(user: <b>address</b>, asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): &<b>mut</b> <a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a> {
    <b>assert</b>!(<a href="confidential_asset.md#0x7_confidential_asset_has_confidential_store">has_confidential_store</a>(user, asset_type), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_CONFIDENTIAL_STORE_NOT_REGISTERED">E_CONFIDENTIAL_STORE_NOT_REGISTERED</a>));
    <b>borrow_global_mut</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>&gt;(<a href="confidential_asset.md#0x7_confidential_asset_get_confidential_store_address">get_confidential_store_address</a>(user, asset_type))
}
</code></pre>



</details>

<a id="0x7_confidential_asset_get_asset_config_signer"></a>

## Function `get_asset_config_signer`

Returns an object for handling the <code><a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a></code>, and returns a signer for it.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_signer">get_asset_config_signer</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_get_asset_config_signer">get_asset_config_signer</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a> {
    <b>let</b> config_ext = &<b>borrow_global</b>&lt;<a href="confidential_asset.md#0x7_confidential_asset_GlobalConfig">GlobalConfig</a>&gt;(@aptos_experimental).extend_ref;
    <b>let</b> config_ext_signer = <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer_for_extending">object::generate_signer_for_extending</a>(config_ext);

    <b>let</b> config_ctor =
        &<a href="../../aptos-framework/doc/object.md#0x1_object_create_named_object">object::create_named_object</a>(&config_ext_signer, <a href="confidential_asset.md#0x7_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type));

    <a href="../../aptos-framework/doc/object.md#0x1_object_generate_signer">object::generate_signer</a>(config_ctor)
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
            &b"<a href="confidential_asset.md#0x7_confidential_asset">confidential_asset</a>::{}::asset_type::{}::<a href="confidential_asset.md#0x7_confidential_asset_ConfidentialStore">ConfidentialStore</a>",
            @aptos_experimental,
            <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&asset_type)
        )
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_construct_asset_config_seed"></a>

## Function `construct_asset_config_seed`

Constructs a unique seed for the <code><a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a></code> object.
As all the <code><a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a></code>'s have the same type, we need to differentiate them by the seed.
NOTE: The seed string is unchanged from the original to maintain address stability.


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_construct_asset_config_seed">construct_asset_config_seed</a>(asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(
        &<a href="../../aptos-framework/../aptos-stdlib/doc/string_utils.md#0x1_string_utils_format2">string_utils::format2</a>(
            &b"<a href="confidential_asset.md#0x7_confidential_asset">confidential_asset</a>::{}::asset_type::{}::<a href="confidential_asset.md#0x7_confidential_asset_AssetConfig">AssetConfig</a>",
            @aptos_experimental,
            <a href="../../aptos-framework/doc/object.md#0x1_object_object_address">object::object_address</a>(&asset_type)
        )
    )
}
</code></pre>



</details>

<a id="0x7_confidential_asset_assert_valid_registration_proof"></a>

## Function `assert_valid_registration_proof`

Asserts the registration proof of knowledge is valid via $\Sigma$-protocol verification.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_registration_proof">assert_valid_registration_proof</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, proof: <a href="confidential_asset.md#0x7_confidential_asset_RegistrationProof">confidential_asset::RegistrationProof</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_registration_proof">assert_valid_registration_proof</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: &CompressedRistretto,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_RegistrationProof">RegistrationProof</a>
) {
    <b>let</b> RegistrationProof::V1 { sigma } = proof;
    <b>let</b> compressed_H = <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed</a>();
    <b>let</b> stmt = <a href="sigma_protocol_registration.md#0x7_sigma_protocol_registration_new_registration_statement">sigma_protocol_registration::new_registration_statement</a>(
        compressed_H, compressed_H.point_decompress(),
        *ek, ek.point_decompress(),
    );
    <b>let</b> session = <a href="sigma_protocol_registration.md#0x7_sigma_protocol_registration_new_session">sigma_protocol_registration::new_session</a>(sender, asset_type);
    <a href="sigma_protocol_registration.md#0x7_sigma_protocol_registration_assert_verifies">sigma_protocol_registration::assert_verifies</a>(&session, &stmt, &sigma);
}
</code></pre>



</details>

<a id="0x7_confidential_asset_assert_valid_withdrawal_proof"></a>

## Function `assert_valid_withdrawal_proof`

Asserts the validity of the <code>withdraw</code> operation.

Checks that the new balance chunks are each in [0, 2^16) via a range proof.
Verifies the $\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{withdraw}$ relation.
Consumes the proof and returns the compressed new balance on success.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_withdrawal_proof">assert_valid_withdrawal_proof</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, amount: u64, current_balance: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>, compressed_auditor_ek: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, proof: <a href="confidential_asset.md#0x7_confidential_asset_WithdrawalProof">confidential_asset::WithdrawalProof</a>): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_withdrawal_proof">assert_valid_withdrawal_proof</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: &CompressedRistretto,
    amount: u64,
    current_balance: &CompressedAvailableBalance,
    compressed_auditor_ek: &Option&lt;CompressedRistretto&gt;,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_WithdrawalProof">WithdrawalProof</a>
): CompressedAvailableBalance {
    <b>let</b> WithdrawalProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma } = proof;
    <a href="confidential_proof.md#0x7_confidential_proof_assert_valid_range_proof">confidential_proof::assert_valid_range_proof</a>(new_balance.get_P(), &zkrp_new_balance);

    // Build base points
    <b>let</b> compressed_G = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>();
    <b>let</b> _G = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>();
    <b>let</b> compressed_H = <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed</a>();
    <b>let</b> _H = compressed_H.point_decompress();

    <b>let</b> v = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u64">ristretto255::new_scalar_from_u64</a>(amount);

    <b>let</b> (aud_ek_compressed, compressed_new_R_aud, new_R_aud) = <b>if</b> (compressed_auditor_ek.is_some()) {
        <b>let</b> aud_ek = *compressed_auditor_ek.borrow();
        (std::option::some(aud_ek), *compressed_new_balance.get_compressed_R_aud(), points_clone(new_balance.get_R_aud()))
    } <b>else</b> {
        (std::option::none(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[])
    };

    <b>let</b> stmt = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_withdrawal_statement">sigma_protocol_withdraw::new_withdrawal_statement</a>(
        compressed_G, _G,
        compressed_H, _H,
        *ek, ek.point_decompress(),
        *current_balance.get_compressed_P(), decompress_points(current_balance.get_compressed_P()),
        *current_balance.get_compressed_R(), decompress_points(current_balance.get_compressed_R()),
        *compressed_new_balance.get_compressed_P(), points_clone(new_balance.get_P()),
        *compressed_new_balance.get_compressed_R(), points_clone(new_balance.get_R()),
        aud_ek_compressed, aud_ek_compressed.map(|p| p.point_decompress()),
        compressed_new_R_aud, new_R_aud,
        v,
    );

    <b>let</b> session = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_session">sigma_protocol_withdraw::new_session</a>(sender, asset_type);
    <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_verifies_withdrawal">sigma_protocol_withdraw::assert_verifies_withdrawal</a>(&session, &stmt, &sigma);
    compressed_new_balance
}
</code></pre>



</details>

<a id="0x7_confidential_asset_assert_valid_transfer_proof"></a>

## Function `assert_valid_transfer_proof`

Asserts the validity of the <code>confidential_transfer</code> operation.

Checks that the new balance and transfer amount chunks are each in [0, 2^16) via range proofs.
Reconstructs full recipient and auditor balances from the sender_amount's C components and
the provided D-only components. This structurally guarantees that C components match across
all parties.
Verifies the $\Sigma$-protocol proof for the $\mathcal{R}^{-}_\mathsf{txfer}$ relation.
Consumes the proof and returns (new_balance, sender_amount, recipient_amount, auditor_amounts).


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_transfer_proof">assert_valid_transfer_proof</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, recipient_addr: <b>address</b>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, sender_ek: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, recip_ek: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, old_balance: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>, auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, has_effective_auditor: bool, num_extra_auditors: u64, proof: <a href="confidential_asset.md#0x7_confidential_asset_TransferProof">confidential_asset::TransferProof</a>): (<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>, <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="confidential_pending_balance.md#0x7_confidential_pending_balance_PendingBalance">confidential_pending_balance::PendingBalance</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_transfer_proof">assert_valid_transfer_proof</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    recipient_addr: <b>address</b>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    sender_ek: &CompressedRistretto,
    recip_ek: &CompressedRistretto,
    old_balance: &CompressedAvailableBalance,
    auditor_eks: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;,
    has_effective_auditor: bool,
    num_extra_auditors: u64,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_TransferProof">TransferProof</a>
): (
    CompressedAvailableBalance,
    PendingBalance,
    PendingBalance,
    <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;PendingBalance&gt;
) {
    <b>let</b> TransferProof::V1 {
        new_balance, compressed_new_balance,
        sender_amount, compressed_sender_amount,
        recipient_amount_R, compressed_recip_amount_R,
        auditor_amount_Ds, compressed_auditor_Rs,
        zkrp_new_balance, zkrp_amount, sigma
    } = proof;

    // Verify the number of auditor D-components in the proof matches the expected auditor count
    // (cheap check before expensive sigma verification)
    <b>assert</b>!(
        auditor_amount_Ds.length() == auditor_eks.length(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="confidential_asset.md#0x7_confidential_asset_E_AUDITOR_COUNT_MISMATCH">E_AUDITOR_COUNT_MISMATCH</a>)
    );

    // Reconstruct full balances from sender_amount's C components and the D-only components.
    // This structurally guarantees C component equality (no explicit check needed).
    <b>let</b> sender_P = sender_amount.get_P();
    <b>let</b> recipient_amount = <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">confidential_pending_balance::new_from_p_and_r</a>(
        sender_P.map_ref(|c| c.point_clone()),
        recipient_amount_R
    );
    <b>let</b> auditor_amounts = auditor_amount_Ds.map(|d| {
        <a href="confidential_pending_balance.md#0x7_confidential_pending_balance_new_from_p_and_r">confidential_pending_balance::new_from_p_and_r</a>(
            sender_P.map_ref(|c| c.point_clone()), d
        )
    });

    <a href="confidential_proof.md#0x7_confidential_proof_assert_valid_range_proof">confidential_proof::assert_valid_range_proof</a>(recipient_amount.get_P(), &zkrp_amount);
    <a href="confidential_proof.md#0x7_confidential_proof_assert_valid_range_proof">confidential_proof::assert_valid_range_proof</a>(new_balance.get_P(), &zkrp_new_balance);

    // Sigma protocol verification
    <b>let</b> compressed_G = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>();
    <b>let</b> _G = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>();
    <b>let</b> compressed_H = <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed</a>();
    <b>let</b> _H = compressed_H.point_decompress();

    // Effective auditor components (<b>if</b> <a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a>)
    <b>let</b> (ek_eff_aud_opt, compressed_new_R_eff_aud, new_R_eff_aud,
         compressed_amount_R_eff_aud, amount_R_eff_aud) =
        <b>if</b> (has_effective_auditor) {
            <b>let</b> eff_aud_ek = auditor_eks[auditor_eks.length() - 1];
            (
                std::option::some(eff_aud_ek),
                *compressed_new_balance.get_compressed_R_aud(), points_clone(new_balance.get_R_aud()),
                compressed_auditor_Rs[compressed_auditor_Rs.length() - 1],
                points_clone(auditor_amounts[auditor_amounts.length() - 1].get_R()),
            )
        } <b>else</b> {
            (std::option::none(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[])
        };

    // Extra auditor components (extras are at indices [0..num_extra) in the auditor vectors)
    <b>let</b> compressed_ek_extra_auds = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> ek_extra_auds = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> compressed_amount_R_extra_auds = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> amount_R_extra_auds = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_extra_auditors) {
        compressed_ek_extra_auds.push_back(auditor_eks[i]);
        ek_extra_auds.push_back(auditor_eks[i].point_decompress());
        compressed_amount_R_extra_auds.push_back(compressed_auditor_Rs[i]);
        amount_R_extra_auds.push_back(points_clone(auditor_amounts[i].get_R()));
        i = i + 1;
    };

    <b>let</b> stmt = <a href="sigma_protocol_transfer.md#0x7_sigma_protocol_transfer_new_transfer_statement">sigma_protocol_transfer::new_transfer_statement</a>(
        compressed_G, _G,
        compressed_H, _H,
        *sender_ek, sender_ek.point_decompress(),
        *recip_ek, recip_ek.point_decompress(),
        *old_balance.get_compressed_P(), decompress_points(old_balance.get_compressed_P()),
        *old_balance.get_compressed_R(), decompress_points(old_balance.get_compressed_R()),
        *compressed_new_balance.get_compressed_P(), points_clone(new_balance.get_P()),
        *compressed_new_balance.get_compressed_R(), points_clone(new_balance.get_R()),
        *compressed_sender_amount.get_compressed_P(), points_clone(sender_amount.get_P()),
        *compressed_sender_amount.get_compressed_R(), points_clone(sender_amount.get_R()),
        compressed_recip_amount_R, points_clone(recipient_amount.get_R()),
        ek_eff_aud_opt, ek_eff_aud_opt.map(|p| p.point_decompress()),
        compressed_new_R_eff_aud, new_R_eff_aud,
        compressed_amount_R_eff_aud, amount_R_eff_aud,
        compressed_ek_extra_auds, ek_extra_auds,
        compressed_amount_R_extra_auds, amount_R_extra_auds,
    );

    <b>let</b> session = <a href="sigma_protocol_transfer.md#0x7_sigma_protocol_transfer_new_session">sigma_protocol_transfer::new_session</a>(
        sender, recipient_addr, asset_type, has_effective_auditor, num_extra_auditors,
    );
    <a href="sigma_protocol_transfer.md#0x7_sigma_protocol_transfer_assert_verifies">sigma_protocol_transfer::assert_verifies</a>(&session, &stmt, &sigma);

    (compressed_new_balance, sender_amount, recipient_amount, auditor_amounts)
}
</code></pre>



</details>

<a id="0x7_confidential_asset_assert_valid_normalization_proof"></a>

## Function `assert_valid_normalization_proof`

Asserts the validity of the <code>normalize</code> operation.

Checks that the new balance chunks are each in [0, 2^16) via a range proof.
Verifies the $\Sigma$-protocol proof for the normalization relation (withdrawal with $v = 0$).
Consumes the proof and returns the compressed new balance on success.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_normalization_proof">assert_valid_normalization_proof</a>(sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, ek: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, current_balance: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>, auditor_ek: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;, proof: <a href="confidential_asset.md#0x7_confidential_asset_NormalizationProof">confidential_asset::NormalizationProof</a>): <a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_normalization_proof">assert_valid_normalization_proof</a>(
    sender: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    ek: &CompressedRistretto,
    current_balance: &CompressedAvailableBalance,
    auditor_ek: &Option&lt;CompressedRistretto&gt;,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_NormalizationProof">NormalizationProof</a>
): CompressedAvailableBalance {
    <b>let</b> NormalizationProof::V1 { new_balance, compressed_new_balance, zkrp_new_balance, sigma } = proof;

    <a href="confidential_proof.md#0x7_confidential_proof_assert_valid_range_proof">confidential_proof::assert_valid_range_proof</a>(new_balance.get_P(), &zkrp_new_balance);

    // Build base points
    <b>let</b> compressed_G = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint_compressed">ristretto255::basepoint_compressed</a>();
    <b>let</b> _G = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_basepoint">ristretto255::basepoint</a>();
    <b>let</b> compressed_H = <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed</a>();
    <b>let</b> _H = compressed_H.point_decompress();

    // Normalization is withdrawal <b>with</b> v = 0
    <b>let</b> v = <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_new_scalar_from_u64">ristretto255::new_scalar_from_u64</a>(0);

    <b>let</b> (aud_ek_compressed, compressed_new_R_aud, new_R_aud) = <b>if</b> (auditor_ek.is_some()) {
        <b>let</b> aud_ek = *auditor_ek.borrow();
        (std::option::some(aud_ek), *compressed_new_balance.get_compressed_R_aud(), points_clone(new_balance.get_R_aud()))
    } <b>else</b> {
        (std::option::none(), <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[])
    };

    <b>let</b> stmt = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_withdrawal_statement">sigma_protocol_withdraw::new_withdrawal_statement</a>(
        compressed_G, _G,
        compressed_H, _H,
        *ek, ek.point_decompress(),
        *current_balance.get_compressed_P(), decompress_points(current_balance.get_compressed_P()),
        *current_balance.get_compressed_R(), decompress_points(current_balance.get_compressed_R()),
        *compressed_new_balance.get_compressed_P(), points_clone(new_balance.get_P()),
        *compressed_new_balance.get_compressed_R(), points_clone(new_balance.get_R()),
        aud_ek_compressed, aud_ek_compressed.map(|p| p.point_decompress()),
        compressed_new_R_aud, new_R_aud,
        v,
    );

    <b>let</b> session = <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_new_session">sigma_protocol_withdraw::new_session</a>(sender, asset_type);
    <a href="sigma_protocol_withdraw.md#0x7_sigma_protocol_withdraw_assert_verifies_normalization">sigma_protocol_withdraw::assert_verifies_normalization</a>(&session, &stmt, &sigma);

    compressed_new_balance
}
</code></pre>



</details>

<a id="0x7_confidential_asset_assert_valid_key_rotation_proof"></a>

## Function `assert_valid_key_rotation_proof`

Asserts the validity of the key rotation proof via $\Sigma$-protocol verification.
Returns (compressed_new_ek, compressed_new_R) on success.


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_key_rotation_proof">assert_valid_key_rotation_proof</a>(owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, asset_type: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;, new_ek: <a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_RistrettoPoint">ristretto255::RistrettoPoint</a>, old_ek: &<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, current_balance: &<a href="confidential_available_balance.md#0x7_confidential_available_balance_CompressedAvailableBalance">confidential_available_balance::CompressedAvailableBalance</a>, proof: <a href="confidential_asset.md#0x7_confidential_asset_KeyRotationProof">confidential_asset::KeyRotationProof</a>): (<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/doc/ristretto255.md#0x1_ristretto255_CompressedRistretto">ristretto255::CompressedRistretto</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="confidential_asset.md#0x7_confidential_asset_assert_valid_key_rotation_proof">assert_valid_key_rotation_proof</a>(
    owner: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    asset_type: Object&lt;<a href="../../aptos-framework/doc/fungible_asset.md#0x1_fungible_asset_Metadata">fungible_asset::Metadata</a>&gt;,
    new_ek: RistrettoPoint,
    old_ek: &CompressedRistretto,
    current_balance: &CompressedAvailableBalance,
    proof: <a href="confidential_asset.md#0x7_confidential_asset_KeyRotationProof">KeyRotationProof</a>
): (CompressedRistretto, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;CompressedRistretto&gt;) {
    <b>let</b> KeyRotationProof::V1 { compressed_new_ek, new_R, compressed_new_R, sigma } = proof;

    <b>let</b> compressed_H = <a href="ristretto255_twisted_elgamal.md#0x7_ristretto255_twisted_elgamal_get_encryption_key_basepoint_compressed">ristretto255_twisted_elgamal::get_encryption_key_basepoint_compressed</a>();

    <b>let</b> stmt = <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_key_rotation_statement">sigma_protocol_key_rotation::new_key_rotation_statement</a>(
        compressed_H, compressed_H.point_decompress(),
        *old_ek, old_ek.point_decompress(),
        compressed_new_ek, new_ek,
        *current_balance.get_compressed_R(), decompress_points(current_balance.get_compressed_R()),
        compressed_new_R, new_R,
    );

    <b>let</b> session = <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_new_session">sigma_protocol_key_rotation::new_session</a>(owner, asset_type);
    <a href="sigma_protocol_key_rotation.md#0x7_sigma_protocol_key_rotation_assert_verifies">sigma_protocol_key_rotation::assert_verifies</a>(&session, &stmt, &sigma);

    (compressed_new_ek, compressed_new_R)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
