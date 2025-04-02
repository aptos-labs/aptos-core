
<a id="0x1_account"></a>

# Module `0x1::account`



-  [Struct `KeyRotation`](#0x1_account_KeyRotation)
-  [Struct `KeyRotationToPublicKey`](#0x1_account_KeyRotationToPublicKey)
-  [Resource `Account`](#0x1_account_Account)
-  [Struct `KeyRotationEvent`](#0x1_account_KeyRotationEvent)
-  [Struct `CoinRegisterEvent`](#0x1_account_CoinRegisterEvent)
-  [Struct `CoinRegister`](#0x1_account_CoinRegister)
-  [Struct `CapabilityOffer`](#0x1_account_CapabilityOffer)
-  [Struct `RotationCapability`](#0x1_account_RotationCapability)
-  [Struct `SignerCapability`](#0x1_account_SignerCapability)
-  [Resource `OriginatingAddress`](#0x1_account_OriginatingAddress)
-  [Struct `RotationProofChallenge`](#0x1_account_RotationProofChallenge)
-  [Struct `RotationCapabilityOfferProofChallenge`](#0x1_account_RotationCapabilityOfferProofChallenge)
-  [Struct `SignerCapabilityOfferProofChallenge`](#0x1_account_SignerCapabilityOfferProofChallenge)
-  [Struct `RotationCapabilityOfferProofChallengeV2`](#0x1_account_RotationCapabilityOfferProofChallengeV2)
-  [Struct `SignerCapabilityOfferProofChallengeV2`](#0x1_account_SignerCapabilityOfferProofChallengeV2)
-  [Enum `AccountPermission`](#0x1_account_AccountPermission)
-  [Constants](#@Constants_0)
-  [Function `check_rotation_permission`](#0x1_account_check_rotation_permission)
-  [Function `check_offering_permission`](#0x1_account_check_offering_permission)
-  [Function `grant_key_rotation_permission`](#0x1_account_grant_key_rotation_permission)
-  [Function `grant_key_offering_permission`](#0x1_account_grant_key_offering_permission)
-  [Function `initialize`](#0x1_account_initialize)
-  [Function `create_account_if_does_not_exist`](#0x1_account_create_account_if_does_not_exist)
-  [Function `create_account`](#0x1_account_create_account)
-  [Function `create_account_unchecked`](#0x1_account_create_account_unchecked)
-  [Function `exists_at`](#0x1_account_exists_at)
-  [Function `resource_exists_at`](#0x1_account_resource_exists_at)
-  [Function `get_guid_next_creation_num`](#0x1_account_get_guid_next_creation_num)
-  [Function `get_sequence_number`](#0x1_account_get_sequence_number)
-  [Function `originating_address`](#0x1_account_originating_address)
-  [Function `ensure_resource_exists`](#0x1_account_ensure_resource_exists)
-  [Function `increment_sequence_number`](#0x1_account_increment_sequence_number)
-  [Function `get_authentication_key`](#0x1_account_get_authentication_key)
-  [Function `rotate_authentication_key_internal`](#0x1_account_rotate_authentication_key_internal)
-  [Function `rotate_authentication_key_call`](#0x1_account_rotate_authentication_key_call)
-  [Function `rotate_authentication_key_from_public_key`](#0x1_account_rotate_authentication_key_from_public_key)
-  [Function `upsert_ed25519_backup_key_on_keyless_account`](#0x1_account_upsert_ed25519_backup_key_on_keyless_account)
    -  [Arguments](#@Arguments_1)
    -  [Aborts](#@Aborts_2)
    -  [Events](#@Events_3)
-  [Function `rotate_authentication_key`](#0x1_account_rotate_authentication_key)
-  [Function `rotate_authentication_key_with_rotation_capability`](#0x1_account_rotate_authentication_key_with_rotation_capability)
-  [Function `offer_rotation_capability`](#0x1_account_offer_rotation_capability)
-  [Function `set_originating_address`](#0x1_account_set_originating_address)
-  [Function `is_rotation_capability_offered`](#0x1_account_is_rotation_capability_offered)
-  [Function `get_rotation_capability_offer_for`](#0x1_account_get_rotation_capability_offer_for)
-  [Function `revoke_rotation_capability`](#0x1_account_revoke_rotation_capability)
-  [Function `revoke_any_rotation_capability`](#0x1_account_revoke_any_rotation_capability)
-  [Function `offer_signer_capability`](#0x1_account_offer_signer_capability)
-  [Function `is_signer_capability_offered`](#0x1_account_is_signer_capability_offered)
-  [Function `get_signer_capability_offer_for`](#0x1_account_get_signer_capability_offer_for)
-  [Function `revoke_signer_capability`](#0x1_account_revoke_signer_capability)
-  [Function `revoke_any_signer_capability`](#0x1_account_revoke_any_signer_capability)
-  [Function `create_authorized_signer`](#0x1_account_create_authorized_signer)
-  [Function `assert_account_resource_with_error`](#0x1_account_assert_account_resource_with_error)
-  [Function `assert_valid_rotation_proof_signature_and_get_auth_key`](#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key)
-  [Function `update_auth_key_and_originating_address_table`](#0x1_account_update_auth_key_and_originating_address_table)
-  [Function `create_resource_address`](#0x1_account_create_resource_address)
-  [Function `create_resource_account`](#0x1_account_create_resource_account)
-  [Function `create_framework_reserved_account`](#0x1_account_create_framework_reserved_account)
-  [Function `create_guid`](#0x1_account_create_guid)
-  [Function `new_event_handle`](#0x1_account_new_event_handle)
-  [Function `register_coin`](#0x1_account_register_coin)
-  [Function `create_signer_with_capability`](#0x1_account_create_signer_with_capability)
-  [Function `get_signer_capability_address`](#0x1_account_get_signer_capability_address)
-  [Function `verify_signed_message`](#0x1_account_verify_signed_message)
-  [Specification](#@Specification_4)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_4_initialize)
    -  [Function `create_account_if_does_not_exist`](#@Specification_4_create_account_if_does_not_exist)
    -  [Function `create_account`](#@Specification_4_create_account)
    -  [Function `create_account_unchecked`](#@Specification_4_create_account_unchecked)
    -  [Function `exists_at`](#@Specification_4_exists_at)
    -  [Function `get_guid_next_creation_num`](#@Specification_4_get_guid_next_creation_num)
    -  [Function `get_sequence_number`](#@Specification_4_get_sequence_number)
    -  [Function `originating_address`](#@Specification_4_originating_address)
    -  [Function `increment_sequence_number`](#@Specification_4_increment_sequence_number)
    -  [Function `get_authentication_key`](#@Specification_4_get_authentication_key)
    -  [Function `rotate_authentication_key_internal`](#@Specification_4_rotate_authentication_key_internal)
    -  [Function `rotate_authentication_key_call`](#@Specification_4_rotate_authentication_key_call)
    -  [Function `rotate_authentication_key_from_public_key`](#@Specification_4_rotate_authentication_key_from_public_key)
    -  [Function `rotate_authentication_key`](#@Specification_4_rotate_authentication_key)
    -  [Function `rotate_authentication_key_with_rotation_capability`](#@Specification_4_rotate_authentication_key_with_rotation_capability)
    -  [Function `offer_rotation_capability`](#@Specification_4_offer_rotation_capability)
    -  [Function `set_originating_address`](#@Specification_4_set_originating_address)
    -  [Function `is_rotation_capability_offered`](#@Specification_4_is_rotation_capability_offered)
    -  [Function `get_rotation_capability_offer_for`](#@Specification_4_get_rotation_capability_offer_for)
    -  [Function `revoke_rotation_capability`](#@Specification_4_revoke_rotation_capability)
    -  [Function `revoke_any_rotation_capability`](#@Specification_4_revoke_any_rotation_capability)
    -  [Function `offer_signer_capability`](#@Specification_4_offer_signer_capability)
    -  [Function `is_signer_capability_offered`](#@Specification_4_is_signer_capability_offered)
    -  [Function `get_signer_capability_offer_for`](#@Specification_4_get_signer_capability_offer_for)
    -  [Function `revoke_signer_capability`](#@Specification_4_revoke_signer_capability)
    -  [Function `revoke_any_signer_capability`](#@Specification_4_revoke_any_signer_capability)
    -  [Function `create_authorized_signer`](#@Specification_4_create_authorized_signer)
    -  [Function `assert_valid_rotation_proof_signature_and_get_auth_key`](#@Specification_4_assert_valid_rotation_proof_signature_and_get_auth_key)
    -  [Function `update_auth_key_and_originating_address_table`](#@Specification_4_update_auth_key_and_originating_address_table)
    -  [Function `create_resource_address`](#@Specification_4_create_resource_address)
    -  [Function `create_resource_account`](#@Specification_4_create_resource_account)
    -  [Function `create_framework_reserved_account`](#@Specification_4_create_framework_reserved_account)
    -  [Function `create_guid`](#@Specification_4_create_guid)
    -  [Function `new_event_handle`](#@Specification_4_new_event_handle)
    -  [Function `register_coin`](#@Specification_4_register_coin)
    -  [Function `create_signer_with_capability`](#@Specification_4_create_signer_with_capability)
    -  [Function `verify_signed_message`](#@Specification_4_verify_signed_message)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;
<b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519">0x1::multi_ed25519</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/multi_key.md#0x1_multi_key">0x1::multi_key</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="permissioned_signer.md#0x1_permissioned_signer">0x1::permissioned_signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/single_key.md#0x1_single_key">0x1::single_key</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_account_KeyRotation"></a>

## Struct `KeyRotation`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="account.md#0x1_account_KeyRotation">KeyRotation</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_KeyRotationToPublicKey"></a>

## Struct `KeyRotationToPublicKey`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="account.md#0x1_account_KeyRotationToPublicKey">KeyRotationToPublicKey</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>verified_public_key_bit_map: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>public_key_scheme: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>old_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_Account"></a>

## Resource `Account`

Resource representing an account.


<pre><code><b>struct</b> <a href="account.md#0x1_account_Account">Account</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>guid_creation_num: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>coin_register_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="account.md#0x1_account_CoinRegisterEvent">account::CoinRegisterEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>key_rotation_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="account.md#0x1_account_KeyRotationEvent">account::KeyRotationEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>rotation_capability_offer: <a href="account.md#0x1_account_CapabilityOffer">account::CapabilityOffer</a>&lt;<a href="account.md#0x1_account_RotationCapability">account::RotationCapability</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>signer_capability_offer: <a href="account.md#0x1_account_CapabilityOffer">account::CapabilityOffer</a>&lt;<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_KeyRotationEvent"></a>

## Struct `KeyRotationEvent`



<pre><code><b>struct</b> <a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_authentication_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_CoinRegisterEvent"></a>

## Struct `CoinRegisterEvent`



<pre><code><b>struct</b> <a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">type_info</a>: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_CoinRegister"></a>

## Struct `CoinRegister`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="account.md#0x1_account_CoinRegister">CoinRegister</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">type_info</a>: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_TypeInfo">type_info::TypeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_CapabilityOffer"></a>

## Struct `CapabilityOffer`



<pre><code><b>struct</b> <a href="account.md#0x1_account_CapabilityOffer">CapabilityOffer</a>&lt;T&gt; <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>for: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationCapability"></a>

## Struct `RotationCapability`



<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationCapability">RotationCapability</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_SignerCapability"></a>

## Struct `SignerCapability`



<pre><code><b>struct</b> <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_OriginatingAddress"></a>

## Resource `OriginatingAddress`

It is easy to fetch the authentication key of an address by simply reading it from the <code><a href="account.md#0x1_account_Account">Account</a></code> struct at that address.
The table in this struct makes it possible to do a reverse lookup: it maps an authentication key, to the address of the account which has that authentication key set.

This mapping is needed when recovering wallets for accounts whose authentication key has been rotated.

For example, imagine a freshly-created wallet with address <code>a</code> and thus also with authentication key <code>a</code>, derived from a PK <code>pk_a</code> with corresponding SK <code>sk_a</code>.
It is easy to recover such a wallet given just the secret key <code>sk_a</code>, since the PK can be derived from the SK, the authentication key can then be derived from the PK, and the address equals the authentication key (since there was no key rotation).

However, if such a wallet rotates its authentication key to <code>b</code> derived from a different PK <code>pk_b</code> with SK <code>sk_b</code>, how would account recovery work?
The recovered address would no longer be 'a'; it would be <code>b</code>, which is incorrect.
This struct solves this problem by mapping the new authentication key <code>b</code> to the original address <code>a</code> and thus helping the wallet software during recovery find the correct address.


<pre><code><b>struct</b> <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>address_map: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<b>address</b>, <b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationProofChallenge"></a>

## Struct `RotationProofChallenge`

This structs stores the challenge message that should be signed during key rotation. First, this struct is
signed by the account owner's current public key, which proves possession of a capability to rotate the key.
Second, this struct is signed by the new public key that the account owner wants to rotate to, which proves
knowledge of this new public key's associated secret key. These two signatures cannot be replayed in another
context because they include the TXN's unique sequence number.


<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>originator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>current_auth_key: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>new_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationCapabilityOfferProofChallenge"></a>

## Struct `RotationCapabilityOfferProofChallenge`

Deprecated struct - newest version is <code><a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a></code>


<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationCapabilityOfferProofChallenge">RotationCapabilityOfferProofChallenge</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_SignerCapabilityOfferProofChallenge"></a>

## Struct `SignerCapabilityOfferProofChallenge`

Deprecated struct - newest version is <code><a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a></code>


<pre><code><b>struct</b> <a href="account.md#0x1_account_SignerCapabilityOfferProofChallenge">SignerCapabilityOfferProofChallenge</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationCapabilityOfferProofChallengeV2"></a>

## Struct `RotationCapabilityOfferProofChallengeV2`

This struct stores the challenge message that should be signed by the source account, when the source account
is delegating its rotation capability to the <code>recipient_address</code>.
This V2 struct adds the <code><a href="chain_id.md#0x1_chain_id">chain_id</a></code> and <code>source_address</code> to the challenge message, which prevents replaying the challenge message.


<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="chain_id.md#0x1_chain_id">chain_id</a>: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>source_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_SignerCapabilityOfferProofChallengeV2"></a>

## Struct `SignerCapabilityOfferProofChallengeV2`



<pre><code><b>struct</b> <a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>source_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_AccountPermission"></a>

## Enum `AccountPermission`



<pre><code>enum <a href="account.md#0x1_account_AccountPermission">AccountPermission</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>KeyRotation</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

<details>
<summary>Offering</summary>


<details>
<summary>Fields</summary>


<dl>
</dl>


</details>

</details>

</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_account_MAX_U64"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>: u128 = 18446744073709551615;
</code></pre>



<a id="0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME"></a>

Scheme identifier used when hashing an account's address together with a seed to derive the address (not the
authentication key) of a resource account. This is an abuse of the notion of a scheme identifier which, for now,
serves to domain separate hashes used to derive resource account addresses from hashes used to derive
authentication keys. Without such separation, an adversary could create (and get a signer for) a resource account
whose address matches an existing address of a MultiEd25519 wallet.


<pre><code><b>const</b> <a href="account.md#0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME">DERIVE_RESOURCE_ACCOUNT_SCHEME</a>: u8 = 255;
</code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_EXISTS"></a>

Account already exists


<pre><code><b>const</b> <a href="account.md#0x1_account_EACCOUNT_ALREADY_EXISTS">EACCOUNT_ALREADY_EXISTS</a>: u64 = 1;
</code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_USED"></a>

An attempt to create a resource account on an account that has a committed transaction


<pre><code><b>const</b> <a href="account.md#0x1_account_EACCOUNT_ALREADY_USED">EACCOUNT_ALREADY_USED</a>: u64 = 16;
</code></pre>



<a id="0x1_account_EACCOUNT_DOES_NOT_EXIST"></a>

Account does not exist


<pre><code><b>const</b> <a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>: u64 = 2;
</code></pre>



<a id="0x1_account_ECANNOT_RESERVED_ADDRESS"></a>

Cannot create account because address is reserved


<pre><code><b>const</b> <a href="account.md#0x1_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>: u64 = 5;
</code></pre>



<a id="0x1_account_ED25519_SCHEME"></a>

Scheme identifier for Ed25519 signatures used to derive authentication keys for Ed25519 public keys.


<pre><code><b>const</b> <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>: u8 = 0;
</code></pre>



<a id="0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM">EEXCEEDED_MAX_GUID_CREATION_NUM</a>: u64 = 20;
</code></pre>



<a id="0x1_account_EINVALID_ACCEPT_ROTATION_CAPABILITY"></a>

The caller does not have a valid rotation capability offer from the other account


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_ACCEPT_ROTATION_CAPABILITY">EINVALID_ACCEPT_ROTATION_CAPABILITY</a>: u64 = 10;
</code></pre>



<a id="0x1_account_EINVALID_ORIGINATING_ADDRESS"></a>

Abort the transaction if the expected originating address is different from the originating address on-chain


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_ORIGINATING_ADDRESS">EINVALID_ORIGINATING_ADDRESS</a>: u64 = 13;
</code></pre>



<a id="0x1_account_EINVALID_PROOF_OF_KNOWLEDGE"></a>

Specified proof of knowledge required to prove ownership of a public key is invalid


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>: u64 = 8;
</code></pre>



<a id="0x1_account_EINVALID_SCHEME"></a>

Specified scheme required to proceed with the smart contract operation - can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1)


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>: u64 = 12;
</code></pre>



<a id="0x1_account_EMALFORMED_AUTHENTICATION_KEY"></a>

The provided authentication key has an invalid length


<pre><code><b>const</b> <a href="account.md#0x1_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>: u64 = 4;
</code></pre>



<a id="0x1_account_ENEW_AUTH_KEY_ALREADY_MAPPED"></a>

The new authentication key already has an entry in the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table


<pre><code><b>const</b> <a href="account.md#0x1_account_ENEW_AUTH_KEY_ALREADY_MAPPED">ENEW_AUTH_KEY_ALREADY_MAPPED</a>: u64 = 21;
</code></pre>



<a id="0x1_account_ENEW_AUTH_KEY_SAME_AS_CURRENT"></a>

The current authentication key and the new authentication key are the same


<pre><code><b>const</b> <a href="account.md#0x1_account_ENEW_AUTH_KEY_SAME_AS_CURRENT">ENEW_AUTH_KEY_SAME_AS_CURRENT</a>: u64 = 22;
</code></pre>



<a id="0x1_account_ENOT_A_KEYLESS_PUBLIC_KEY"></a>

The provided public key is not a single Keyless public key


<pre><code><b>const</b> <a href="account.md#0x1_account_ENOT_A_KEYLESS_PUBLIC_KEY">ENOT_A_KEYLESS_PUBLIC_KEY</a>: u64 = 25;
</code></pre>



<a id="0x1_account_ENOT_THE_ORIGINAL_PUBLIC_KEY"></a>

The provided public key is not the original public key for the account


<pre><code><b>const</b> <a href="account.md#0x1_account_ENOT_THE_ORIGINAL_PUBLIC_KEY">ENOT_THE_ORIGINAL_PUBLIC_KEY</a>: u64 = 26;
</code></pre>



<a id="0x1_account_ENO_ACCOUNT_PERMISSION"></a>

Current permissioned signer cannot perform the privilaged operations.


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_ACCOUNT_PERMISSION">ENO_ACCOUNT_PERMISSION</a>: u64 = 23;
</code></pre>



<a id="0x1_account_ENO_CAPABILITY"></a>

The caller does not have a digital-signature-based capability to call this function


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_CAPABILITY">ENO_CAPABILITY</a>: u64 = 9;
</code></pre>



<a id="0x1_account_ENO_SIGNER_CAPABILITY_OFFERED"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_SIGNER_CAPABILITY_OFFERED">ENO_SIGNER_CAPABILITY_OFFERED</a>: u64 = 19;
</code></pre>



<a id="0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER"></a>

The specified rotation capability offer does not exist at the specified offerer address


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>: u64 = 18;
</code></pre>



<a id="0x1_account_ENO_SUCH_SIGNER_CAPABILITY"></a>

The signer capability offer doesn't exist at the given address


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>: u64 = 14;
</code></pre>



<a id="0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS"></a>

Address to create is not a valid reserved address for Aptos framework


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS">ENO_VALID_FRAMEWORK_RESERVED_ADDRESS</a>: u64 = 11;
</code></pre>



<a id="0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST"></a>

Offerer address doesn't exist


<pre><code><b>const</b> <a href="account.md#0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST">EOFFERER_ADDRESS_DOES_NOT_EXIST</a>: u64 = 17;
</code></pre>



<a id="0x1_account_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code><b>const</b> <a href="account.md#0x1_account_EOUT_OF_GAS">EOUT_OF_GAS</a>: u64 = 6;
</code></pre>



<a id="0x1_account_ERESOURCE_ACCCOUNT_EXISTS"></a>

An attempt to create a resource account on a claimed account


<pre><code><b>const</b> <a href="account.md#0x1_account_ERESOURCE_ACCCOUNT_EXISTS">ERESOURCE_ACCCOUNT_EXISTS</a>: u64 = 15;
</code></pre>



<a id="0x1_account_ESEQUENCE_NUMBER_TOO_BIG"></a>

Sequence number exceeds the maximum value for a u64


<pre><code><b>const</b> <a href="account.md#0x1_account_ESEQUENCE_NUMBER_TOO_BIG">ESEQUENCE_NUMBER_TOO_BIG</a>: u64 = 3;
</code></pre>



<a id="0x1_account_EUNRECOGNIZED_SCHEME"></a>

Specified scheme is not recognized. Should be ED25519_SCHEME(0), MULTI_ED25519_SCHEME(1), SINGLE_KEY_SCHEME(2), or MULTI_KEY_SCHEME(3).


<pre><code><b>const</b> <a href="account.md#0x1_account_EUNRECOGNIZED_SCHEME">EUNRECOGNIZED_SCHEME</a>: u64 = 24;
</code></pre>



<a id="0x1_account_EWRONG_CURRENT_PUBLIC_KEY"></a>

Specified current public key is not correct


<pre><code><b>const</b> <a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>: u64 = 7;
</code></pre>



<a id="0x1_account_MAX_GUID_CREATION_NUM"></a>

Explicitly separate the GUID space between Object and Account to prevent accidental overlap.


<pre><code><b>const</b> <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">MAX_GUID_CREATION_NUM</a>: u64 = 1125899906842624;
</code></pre>



<a id="0x1_account_MULTI_ED25519_SCHEME"></a>

Scheme identifier for MultiEd25519 signatures used to derive authentication keys for MultiEd25519 public keys.


<pre><code><b>const</b> <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>: u8 = 1;
</code></pre>



<a id="0x1_account_MULTI_KEY_SCHEME"></a>

Scheme identifier for multi key public keys used to derive authentication keys for multi key public keys.


<pre><code><b>const</b> <a href="account.md#0x1_account_MULTI_KEY_SCHEME">MULTI_KEY_SCHEME</a>: u8 = 3;
</code></pre>



<a id="0x1_account_SINGLE_KEY_SCHEME"></a>

Scheme identifier for single key public keys used to derive authentication keys for single key public keys.


<pre><code><b>const</b> <a href="account.md#0x1_account_SINGLE_KEY_SCHEME">SINGLE_KEY_SCHEME</a>: u8 = 2;
</code></pre>



<a id="0x1_account_ZERO_AUTH_KEY"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a id="0x1_account_check_rotation_permission"></a>

## Function `check_rotation_permission`

Permissions


<pre><code><b>fun</b> <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">permissioned_signer::check_permission_exists</a>(s, AccountPermission::KeyRotation {}),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account.md#0x1_account_ENO_ACCOUNT_PERMISSION">ENO_ACCOUNT_PERMISSION</a>),
    );
}
</code></pre>



</details>

<a id="0x1_account_check_offering_permission"></a>

## Function `check_offering_permission`



<pre><code><b>fun</b> <a href="account.md#0x1_account_check_offering_permission">check_offering_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="account.md#0x1_account_check_offering_permission">check_offering_permission</a>(s: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="permissioned_signer.md#0x1_permissioned_signer_check_permission_exists">permissioned_signer::check_permission_exists</a>(s, AccountPermission::Offering {}),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account.md#0x1_account_ENO_ACCOUNT_PERMISSION">ENO_ACCOUNT_PERMISSION</a>),
    );
}
</code></pre>



</details>

<a id="0x1_account_grant_key_rotation_permission"></a>

## Function `grant_key_rotation_permission`

Grant permission to perform key rotations on behalf of the master signer.

This is **extremely dangerous** and should be granted only when it's absolutely needed.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_grant_key_rotation_permission">grant_key_rotation_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_grant_key_rotation_permission">grant_key_rotation_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">permissioned_signer::authorize_unlimited</a>(master, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>, AccountPermission::KeyRotation {})
}
</code></pre>



</details>

<a id="0x1_account_grant_key_offering_permission"></a>

## Function `grant_key_offering_permission`

Grant permission to use offered address's signer on behalf of the master signer.

This is **extremely dangerous** and should be granted only when it's absolutely needed.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_grant_key_offering_permission">grant_key_offering_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_grant_key_offering_permission">grant_key_offering_permission</a>(master: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="permissioned_signer.md#0x1_permissioned_signer_authorize_unlimited">permissioned_signer::authorize_unlimited</a>(master, <a href="permissioned_signer.md#0x1_permissioned_signer">permissioned_signer</a>, AccountPermission::Offering {})
}
</code></pre>



</details>

<a id="0x1_account_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>move_to</b>(aptos_framework, <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> {
        address_map: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
    });
}
</code></pre>



</details>

<a id="0x1_account_create_account_if_does_not_exist"></a>

## Function `create_account_if_does_not_exist`



<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>) {
    <b>if</b> (!<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(account_address)) {
        <b>assert</b>!(
            account_address != @vm_reserved && account_address != @aptos_framework && account_address != @aptos_token,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>)
        );
        <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(account_address);
    }
}
</code></pre>



</details>

<a id="0x1_account_create_account"></a>

## Function `create_account`

Publishes a new <code><a href="account.md#0x1_account_Account">Account</a></code> resource under <code>new_address</code>. A signer representing <code>new_address</code>
is returned. This way, the caller of this function can publish additional resources under
<code>new_address</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    // there cannot be an <a href="account.md#0x1_account_Account">Account</a> resource under new_addr already.
    <b>assert</b>!(!<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(new_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="account.md#0x1_account_EACCOUNT_ALREADY_EXISTS">EACCOUNT_ALREADY_EXISTS</a>));
    // NOTE: @core_resources gets created via a `create_account` call, so we do not <b>include</b> it below.
    <b>assert</b>!(
        new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>)
    );
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        <a href="create_signer.md#0x1_create_signer">create_signer</a>(new_address)
    } <b>else</b> {
        <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address)
    }
}
</code></pre>



</details>

<a id="0x1_account_create_account_unchecked"></a>

## Function `create_account_unchecked`



<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> new_account = <a href="create_signer.md#0x1_create_signer">create_signer</a>(new_address);
    <b>let</b> authentication_key = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&new_address);
    <b>assert</b>!(
        authentication_key.length() == 32,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>)
    );

    <b>let</b> guid_creation_num = 0;

    <b>let</b> guid_for_coin = <a href="guid.md#0x1_guid_create">guid::create</a>(new_address, &<b>mut</b> guid_creation_num);
    <b>let</b> coin_register_events = <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>&lt;<a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a>&gt;(guid_for_coin);

    <b>let</b> guid_for_rotation = <a href="guid.md#0x1_guid_create">guid::create</a>(new_address, &<b>mut</b> guid_creation_num);
    <b>let</b> key_rotation_events = <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>&lt;<a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a>&gt;(guid_for_rotation);

    <b>move_to</b>(
        &new_account,
        <a href="account.md#0x1_account_Account">Account</a> {
            authentication_key,
            sequence_number: 0,
            guid_creation_num,
            coin_register_events,
            key_rotation_events,
            rotation_capability_offer: <a href="account.md#0x1_account_CapabilityOffer">CapabilityOffer</a> { for: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() },
            signer_capability_offer: <a href="account.md#0x1_account_CapabilityOffer">CapabilityOffer</a> { for: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() },
        }
    );

    new_account
}
</code></pre>



</details>

<a id="0x1_account_exists_at"></a>

## Function `exists_at`

Returns whether an account exists at <code>addr</code>.

When the <code>default_account_resource</code> feature flag is enabled:
- Always returns true, indicating that any address can be treated as a valid account
- This reflects a change in the account model where accounts are now considered to exist implicitly
- The sequence number and other account properties will return default values (0) for addresses without an Account resource

When the feature flag is disabled:
- Returns true only if an Account resource exists at <code>addr</code>
- This is the legacy behavior where accounts must be explicitly created


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_exists_at">exists_at</a>(addr: <b>address</b>): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>() || <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_account_resource_exists_at"></a>

## Function `resource_exists_at`

Returns whether an Account resource exists at <code>addr</code>.

Unlike <code>exists_at</code>, this function strictly checks for the presence of the Account resource,
regardless of the <code>default_account_resource</code> feature flag.

This is useful for operations that specifically need to know if the Account resource
has been created, rather than just whether the address can be treated as an account.


<pre><code><b>fun</b> <a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(addr: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr)
}
</code></pre>



</details>

<a id="0x1_account_get_guid_next_creation_num"></a>

## Function `get_guid_next_creation_num`

Returns the next GUID creation number for <code>addr</code>.

When the <code>default_account_resource</code> feature flag is enabled:
- Returns 0 for addresses without an Account resource
- This allows GUID creation for previously non-existent accounts
- The first GUID created will start the sequence from 0

When the feature flag is disabled:
- Aborts if no Account resource exists at <code>addr</code>


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_guid_next_creation_num">get_guid_next_creation_num</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_guid_next_creation_num">get_guid_next_creation_num</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>if</b> (<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(addr)) {
        <a href="account.md#0x1_account_Account">Account</a>[addr].guid_creation_num
    } <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        0
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>)
    }
}
</code></pre>



</details>

<a id="0x1_account_get_sequence_number"></a>

## Function `get_sequence_number`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>if</b> (<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(addr)) {
        <a href="account.md#0x1_account_Account">Account</a>[addr].sequence_number
    } <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        0
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>)
    }
}
</code></pre>



</details>

<a id="0x1_account_originating_address"></a>

## Function `originating_address`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_originating_address">originating_address</a>(auth_key: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_originating_address">originating_address</a>(auth_key: <b>address</b>): Option&lt;<b>address</b>&gt; <b>acquires</b> <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> {
    <b>let</b> address_map_ref = &<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[@aptos_framework].address_map;
    <b>if</b> (address_map_ref.contains(auth_key)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*address_map_ref.borrow(auth_key))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_account_ensure_resource_exists"></a>

## Function `ensure_resource_exists`



<pre><code><b>fun</b> <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(addr: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a>{
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(addr);
    } <b>else</b> {
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    }
}
</code></pre>



</details>

<a id="0x1_account_increment_sequence_number"></a>

## Function `increment_sequence_number`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(addr);
    <b>let</b> sequence_number = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[addr].sequence_number;

    <b>assert</b>!(
        (*sequence_number <b>as</b> u128) &lt; <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="account.md#0x1_account_ESEQUENCE_NUMBER_TOO_BIG">ESEQUENCE_NUMBER_TOO_BIG</a>)
    );

    *sequence_number = *sequence_number + 1;
}
</code></pre>



</details>

<a id="0x1_account_get_authentication_key"></a>

## Function `get_authentication_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_authentication_key">get_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_authentication_key">get_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>if</b> (<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(addr)) {
        <a href="account.md#0x1_account_Account">Account</a>[addr].authentication_key
    } <b>else</b> <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&addr)
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>)
    }
}
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_internal"></a>

## Function `rotate_authentication_key_internal`

This function is used to rotate a resource account's authentication key to <code>new_auth_key</code>. This is done in
many contexts:
1. During normal key rotation via <code>rotate_authentication_key</code> or <code>rotate_authentication_key_call</code>
2. During resource account initialization so that no private key can control the resource account
3. During multisig_v2 account creation


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(addr);
    <b>assert</b>!(
        new_auth_key.length() == 32,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>)
    );
    <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[addr];
    account_resource.authentication_key = new_auth_key;
}
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_call"></a>

## Function `rotate_authentication_key_call`

Private entry function for key rotation that allows the signer to update their authentication key.
Note that this does not update the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table because the <code>new_auth_key</code> is not "verified": it
does not come with a proof-of-knowledge of the underlying SK. Nonetheless, we need this functionality due to
the introduction of non-standard key algorithms, such as passkeys, which cannot produce proofs-of-knowledge in
the format expected in <code>rotate_authentication_key</code>.

If you'd like to followup with updating the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table, you can call
<code><a href="account.md#0x1_account_set_originating_address">set_originating_address</a>()</code>.


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>, new_auth_key);
}
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_from_public_key"></a>

## Function `rotate_authentication_key_from_public_key`

Private entry function for key rotation that allows the signer to update their authentication key from a given public key.
This function will abort if the scheme is not recognized or if new_public_key_bytes is not a valid public key for the given scheme.

Note: This function does not update the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table.


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_from_public_key">rotate_authentication_key_from_public_key</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, scheme: u8, new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_from_public_key">rotate_authentication_key_from_public_key</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, scheme: u8, new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[addr];
    <b>let</b> old_auth_key = account_resource.authentication_key;
    <b>let</b> new_auth_key;
    <b>if</b> (scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) {
        <b>let</b> from_pk = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(new_public_key_bytes);
        new_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&from_pk);
    } <b>else</b> <b>if</b> (scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) {
        <b>let</b> from_pk = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(new_public_key_bytes);
        new_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&from_pk);
    } <b>else</b> <b>if</b> (scheme == <a href="account.md#0x1_account_SINGLE_KEY_SCHEME">SINGLE_KEY_SCHEME</a>) {
        new_auth_key = <a href="../../aptos-stdlib/doc/single_key.md#0x1_single_key_new_public_key_from_bytes">single_key::new_public_key_from_bytes</a>(new_public_key_bytes).to_authentication_key();
    } <b>else</b> <b>if</b> (scheme == <a href="account.md#0x1_account_MULTI_KEY_SCHEME">MULTI_KEY_SCHEME</a>) {
        new_auth_key = <a href="../../aptos-stdlib/doc/multi_key.md#0x1_multi_key_new_public_key_from_bytes">multi_key::new_public_key_from_bytes</a>(new_public_key_bytes).to_authentication_key();
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EUNRECOGNIZED_SCHEME">EUNRECOGNIZED_SCHEME</a>)
    };
    <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>, new_auth_key);
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="account.md#0x1_account_KeyRotationToPublicKey">KeyRotationToPublicKey</a> {
        <a href="account.md#0x1_account">account</a>: addr,
        // Set verified_public_key_bit_map <b>to</b> [0x00, 0x00, 0x00, 0x00] <b>as</b> the <b>public</b> key(s) are not verified
        verified_public_key_bit_map: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0x00, 0x00, 0x00, 0x00],
        public_key_scheme: scheme,
        public_key: new_public_key_bytes,
        old_auth_key,
        new_auth_key,
    });
}
</code></pre>



</details>

<a id="0x1_account_upsert_ed25519_backup_key_on_keyless_account"></a>

## Function `upsert_ed25519_backup_key_on_keyless_account`

Upserts an ED25519 backup key to an account that has a keyless public key as its original public key by converting the account's authentication key
to a multi-key of the original keyless public key and the new backup key that requires 1 signature from either key to authenticate.
This function takes a the account's original keyless public key and a ED25519 backup public key and rotates the account's authentication key to a multi-key of
the original keyless public key and the new backup key that requires 1 signature from either key to authenticate.

Note: This function emits a <code>KeyRotationToMultiPublicKey</code> event marking both keys as verified since the keyless public key
is the original public key of the account and the new backup key has been validated via verifying the challenge signed by the new backup key.


<a id="@Arguments_1"></a>

### Arguments

* <code><a href="account.md#0x1_account">account</a></code> - The signer representing the keyless account
* <code>keyless_public_key</code> - The original keyless public key of the account (wrapped in an AnyPublicKey)
* <code>backup_public_key</code> - The ED25519 public key to add as a backup
* <code>backup_key_proof</code> - A signature from the backup key proving ownership


<a id="@Aborts_2"></a>

### Aborts

* If the any of inputs deserialize incorrectly
* If the provided public key is not a keyless public key
* If the keyless public key is not the original public key of the account
* If the backup key proof signature is invalid


<a id="@Events_3"></a>

### Events

* Emits a <code>KeyRotationToMultiPublicKey</code> event with the new multi-key configuration


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_upsert_ed25519_backup_key_on_keyless_account">upsert_ed25519_backup_key_on_keyless_account</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, keyless_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, backup_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, backup_key_proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_upsert_ed25519_backup_key_on_keyless_account">upsert_ed25519_backup_key_on_keyless_account</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, keyless_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, backup_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, backup_key_proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    // Check that the provided <b>public</b> key is a <a href="../../aptos-stdlib/doc/keyless.md#0x1_keyless">keyless</a> <b>public</b> key
    <b>let</b> keyless_single_key = <a href="../../aptos-stdlib/doc/single_key.md#0x1_single_key_new_public_key_from_bytes">single_key::new_public_key_from_bytes</a>(keyless_public_key);
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/single_key.md#0x1_single_key_is_keyless_or_federated_keyless_public_key">single_key::is_keyless_or_federated_keyless_public_key</a>(&keyless_single_key), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ENOT_A_KEYLESS_PUBLIC_KEY">ENOT_A_KEYLESS_PUBLIC_KEY</a>));

    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[addr];
    <b>let</b> old_auth_key = account_resource.authentication_key;

    // Check that the provided <b>public</b> key is original <b>public</b> key of the <a href="account.md#0x1_account">account</a> by comparing
    // its authentication key <b>to</b> the <a href="account.md#0x1_account">account</a> <b>address</b>.
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&addr) == keyless_single_key.to_authentication_key(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ENOT_THE_ORIGINAL_PUBLIC_KEY">ENOT_THE_ORIGINAL_PUBLIC_KEY</a>)
    );

    <b>let</b> curr_auth_key_as_address = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(old_auth_key);
    <b>let</b> challenge = <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> {
        sequence_number: account_resource.sequence_number,
        originator: addr,
        current_auth_key: curr_auth_key_as_address,
        new_public_key: backup_public_key,
    };

    // Assert the challenges signed by the provided backup key is valid
    <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(
        <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>,
        backup_public_key,
        backup_key_proof,
        &challenge
    );

    // Get the backup key <b>as</b> a single key
    <b>let</b> backup_key_ed25519 = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(backup_public_key);
    <b>let</b> backup_key_as_single_key = <a href="../../aptos-stdlib/doc/single_key.md#0x1_single_key_from_ed25519_public_key_unvalidated">single_key::from_ed25519_public_key_unvalidated</a>(backup_key_ed25519);

    <b>let</b> new_public_key = <a href="../../aptos-stdlib/doc/multi_key.md#0x1_multi_key_new_multi_key_from_single_keys">multi_key::new_multi_key_from_single_keys</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[keyless_single_key, backup_key_as_single_key], 1);
    <b>let</b> new_auth_key = new_public_key.to_authentication_key();

    // Rotate the authentication key <b>to</b> the new multi key <b>public</b> key
    <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>, new_auth_key);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="account.md#0x1_account_KeyRotationToPublicKey">KeyRotationToPublicKey</a> {
        <a href="account.md#0x1_account">account</a>: addr,
        // This marks that both the <a href="../../aptos-stdlib/doc/keyless.md#0x1_keyless">keyless</a> <b>public</b> key and the new backup key are verified
        // The <a href="../../aptos-stdlib/doc/keyless.md#0x1_keyless">keyless</a> <b>public</b> key is the original <b>public</b> key of the <a href="account.md#0x1_account">account</a> and the new backup key
        // <b>has</b> been validated via verifying the challenge signed by the new backup key.
        // Represents the bitmap 0b11000000000000000000000000000000
        verified_public_key_bit_map: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0xC0, 0x00, 0x00, 0x00],
        public_key_scheme: <a href="account.md#0x1_account_MULTI_KEY_SCHEME">MULTI_KEY_SCHEME</a>,
        public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&new_public_key),
        old_auth_key,
        new_auth_key,
    });
}
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key"></a>

## Function `rotate_authentication_key`

Generic authentication key rotation function that allows the user to rotate their authentication key from any scheme to any scheme.
To authorize the rotation, we need two signatures:
- the first signature <code>cap_rotate_key</code> refers to the signature by the account owner's current key on a valid <code><a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a></code>,
demonstrating that the user intends to and has the capability to rotate the authentication key of this account;
- the second signature <code>cap_update_table</code> refers to the signature by the new key (that the account owner wants to rotate to) on a
valid <code><a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a></code>, demonstrating that the user owns the new private key, and has the authority to update the
<code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> map with the new address mapping <code>&lt;new_address, originating_address&gt;</code>.
To verify these two signatures, we need their corresponding public key and public key scheme: we use <code>from_scheme</code> and <code>from_public_key_bytes</code>
to verify <code>cap_rotate_key</code>, and <code>to_scheme</code> and <code>to_public_key_bytes</code> to verify <code>cap_update_table</code>.
A scheme of 0 refers to an Ed25519 key and a scheme of 1 refers to Multi-Ed25519 keys.
<code>originating <b>address</b></code> refers to an account's original/first address.

Here is an example attack if we don't ask for the second signature <code>cap_update_table</code>:
Alice has rotated her account <code>addr_a</code> to <code>new_addr_a</code>. As a result, the following entry is created, to help Alice when recovering her wallet:
<code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[new_addr_a]</code> -> <code>addr_a</code>
Alice has had a bad day: her laptop blew up and she needs to reset her account on a new one.
(Fortunately, she still has her secret key <code>new_sk_a</code> associated with her new address <code>new_addr_a</code>, so she can do this.)

But Bob likes to mess with Alice.
Bob creates an account <code>addr_b</code> and maliciously rotates it to Alice's new address <code>new_addr_a</code>. Since we are no longer checking a PoK,
Bob can easily do this.

Now, the table will be updated to make Alice's new address point to Bob's address: <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[new_addr_a]</code> -> <code>addr_b</code>.
When Alice recovers her account, her wallet will display the attacker's address (Bob's) <code>addr_b</code> as her address.
Now Alice will give <code>addr_b</code> to everyone to pay her, but the money will go to Bob.

Because we ask for a valid <code>cap_update_table</code>, this kind of attack is not possible. Bob would not have the secret key of Alice's address
to rotate his address to Alice's address in the first place.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key">rotate_authentication_key</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from_scheme: u8, from_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_rotate_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key">rotate_authentication_key</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    from_scheme: u8,
    from_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    to_scheme: u8,
    to_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    cap_rotate_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a>, <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(addr);
    <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[addr];
    <b>let</b> old_auth_key = account_resource.authentication_key;
    // Verify the given `from_public_key_bytes` matches this <a href="account.md#0x1_account">account</a>'s current authentication key.
    <b>if</b> (from_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) {
        <b>let</b> from_pk = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(from_public_key_bytes);
        <b>let</b> from_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&from_pk);
        <b>assert</b>!(
            account_resource.authentication_key == from_auth_key,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)
        );
    } <b>else</b> <b>if</b> (from_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) {
        <b>let</b> from_pk = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(from_public_key_bytes);
        <b>let</b> from_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&from_pk);
        <b>assert</b>!(
            account_resource.authentication_key == from_auth_key,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)
        );
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)
    };

    // Construct a valid `<a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>` that `cap_rotate_key` and `cap_update_table` will validate against.
    <b>let</b> curr_auth_key_as_address = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(account_resource.authentication_key);
    <b>let</b> challenge = <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> {
        sequence_number: account_resource.sequence_number,
        originator: addr,
        current_auth_key: curr_auth_key_as_address,
        new_public_key: to_public_key_bytes,
    };

    // Assert the challenges signed by the current and new keys are valid
    <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(
        from_scheme,
        from_public_key_bytes,
        cap_rotate_key,
        &challenge
    );
    <b>let</b> new_auth_key = <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(
        to_scheme,
        to_public_key_bytes,
        cap_update_table,
        &challenge
    );

    // Update the `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>` <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.
    <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(addr, account_resource, new_auth_key);

    <b>let</b> verified_public_key_bit_map;
    <b>if</b> (to_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) {
        // Set verified_public_key_bit_map <b>to</b> [0x80, 0x00, 0x00, 0x00] <b>as</b> the <b>public</b> key is verified and there is only one <b>public</b> key.
        verified_public_key_bit_map = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0x80, 0x00, 0x00, 0x00];
    } <b>else</b> {
        // The new key is a multi-<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">ed25519</a> key, so set the verified_public_key_bit_map <b>to</b> the signature bitmap.
        <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&cap_update_table);
        verified_public_key_bit_map = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(&cap_update_table, len - 4, len);
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="account.md#0x1_account_KeyRotationToPublicKey">KeyRotationToPublicKey</a> {
        <a href="account.md#0x1_account">account</a>: addr,
        verified_public_key_bit_map,
        public_key_scheme: to_scheme,
        public_key: to_public_key_bytes,
        old_auth_key,
        new_auth_key,
    });
}
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_with_rotation_capability"></a>

## Function `rotate_authentication_key_with_rotation_capability`



<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_with_rotation_capability">rotate_authentication_key_with_rotation_capability</a>(delegate_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_cap_offerer_address: <b>address</b>, new_scheme: u8, new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_with_rotation_capability">rotate_authentication_key_with_rotation_capability</a>(
    delegate_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    rotation_cap_offerer_address: <b>address</b>,
    new_scheme: u8,
    new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a>, <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> {
    <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(delegate_signer);
    <b>assert</b>!(<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(rotation_cap_offerer_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST">EOFFERER_ADDRESS_DOES_NOT_EXIST</a>));

    // Check that there <b>exists</b> a rotation <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> offer at the offerer's <a href="account.md#0x1_account">account</a> resource for the delegate.
    <b>let</b> delegate_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegate_signer);
    <b>let</b> offerer_account_resource = &<a href="account.md#0x1_account_Account">Account</a>[rotation_cap_offerer_address];
    <b>let</b> old_auth_key = offerer_account_resource.authentication_key;
    <b>assert</b>!(
        offerer_account_resource.rotation_capability_offer.for.contains(&delegate_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>)
    );

    <b>let</b> curr_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(offerer_account_resource.authentication_key);
    <b>let</b> challenge = <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> {
        sequence_number: <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(delegate_address),
        originator: rotation_cap_offerer_address,
        current_auth_key: curr_auth_key,
        new_public_key: new_public_key_bytes,
    };

    // Verifies that the `<a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>` from above is signed under the new <b>public</b> key that we are rotating <b>to</b>.        l
    <b>let</b> new_auth_key = <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(
        new_scheme,
        new_public_key_bytes,
        cap_update_table,
        &challenge
    );

    // Update the `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>` <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, so we can find the originating <b>address</b> using the new <b>address</b>.
    <b>let</b> offerer_account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[rotation_cap_offerer_address];
    <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(
        rotation_cap_offerer_address,
        offerer_account_resource,
        new_auth_key
    );

    <b>let</b> verified_public_key_bit_map;
    <b>if</b> (new_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) {
        // Set verified_public_key_bit_map <b>to</b> [0x80, 0x00, 0x00, 0x00] <b>as</b> the <b>public</b> key is verified and there is only one <b>public</b> key.
        verified_public_key_bit_map = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0x80, 0x00, 0x00, 0x00];
    } <b>else</b> {
        // The new key is a multi-<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">ed25519</a> key, so set the verified_public_key_bit_map <b>to</b> the signature bitmap.
        <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&cap_update_table);
        verified_public_key_bit_map = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_slice">vector::slice</a>(&cap_update_table, len - 4, len);
    };

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="account.md#0x1_account_KeyRotationToPublicKey">KeyRotationToPublicKey</a> {
        <a href="account.md#0x1_account">account</a>: rotation_cap_offerer_address,
        verified_public_key_bit_map,
        public_key_scheme: new_scheme,
        public_key: new_public_key_bytes,
        old_auth_key,
        new_auth_key,
    });
}
</code></pre>



</details>

<a id="0x1_account_offer_rotation_capability"></a>

## Function `offer_rotation_capability`

Offers rotation capability on behalf of <code><a href="account.md#0x1_account">account</a></code> to the account at address <code>recipient_address</code>.
An account can delegate its rotation capability to only one other address at one time. If the account
has an existing rotation capability offer, calling this function will update the rotation capability offer with
the new <code>recipient_address</code>.
Here, <code>rotation_capability_sig_bytes</code> signature indicates that this key rotation is authorized by the account owner,
and prevents the classic "time-of-check time-of-use" attack.
For example, users usually rely on what the wallet displays to them as the transaction's outcome. Consider a contract that with 50% probability
(based on the current timestamp in Move), rotates somebody's key. The wallet might be unlucky and get an outcome where nothing is rotated,
incorrectly telling the user nothing bad will happen. But when the transaction actually gets executed, the attacker gets lucky and
the execution path triggers the account key rotation.
We prevent such attacks by asking for this extra signature authorizing the key rotation.

@param rotation_capability_sig_bytes is the signature by the account owner's key on <code><a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a></code>.
@param account_scheme is the scheme of the account (ed25519 or multi_ed25519).
@param account_public_key_bytes is the public key of the account owner.
@param recipient_address is the address of the recipient of the rotation capability - note that if there's an existing rotation capability
offer, calling this function will replace the previous <code>recipient_address</code> upon successful verification.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_rotation_capability">offer_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_rotation_capability">offer_rotation_capability</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    rotation_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    account_scheme: u8,
    account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recipient_address: <b>address</b>,
) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(addr);
    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(recipient_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));

    // proof that this <a href="account.md#0x1_account">account</a> intends <b>to</b> delegate its rotation <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> <b>to</b> another <a href="account.md#0x1_account">account</a>
    <b>let</b> account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[addr];
    <b>let</b> proof_challenge = <a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a> {
        <a href="chain_id.md#0x1_chain_id">chain_id</a>: <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>(),
        sequence_number: account_resource.sequence_number,
        source_address: addr,
        recipient_address,
    };

    // verify the signature on `<a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a>` by the <a href="account.md#0x1_account">account</a> owner
    <b>if</b> (account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) {
        <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key_bytes);
        <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&pubkey);
        <b>assert</b>!(
            account_resource.authentication_key == expected_auth_key,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)
        );

        <b>let</b> rotation_capability_sig = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_signature_from_bytes">ed25519::new_signature_from_bytes</a>(rotation_capability_sig_bytes);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict_t">ed25519::signature_verify_strict_t</a>(&rotation_capability_sig, &pubkey, proof_challenge),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)
        );
    } <b>else</b> <b>if</b> (account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) {
        <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key_bytes);
        <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&pubkey);
        <b>assert</b>!(
            account_resource.authentication_key == expected_auth_key,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)
        );

        <b>let</b> rotation_capability_sig = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">multi_ed25519::new_signature_from_bytes</a>(rotation_capability_sig_bytes);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">multi_ed25519::signature_verify_strict_t</a>(&rotation_capability_sig, &pubkey, proof_challenge),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)
        );
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)
    };

    // <b>update</b> the existing rotation <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> offer or put in a new rotation <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> offer for the current <a href="account.md#0x1_account">account</a>
    account_resource.rotation_capability_offer.for.swap_or_fill(recipient_address);
}
</code></pre>



</details>

<a id="0x1_account_set_originating_address"></a>

## Function `set_originating_address`

For the given account, add an entry to <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table mapping the account's
authentication key to the account's address.

Can be used as a followup to <code><a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>()</code> to reconcile the
<code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table, or to establish a mapping for a new account that has not yet had
its authentication key rotated.

Aborts if there is already an entry in the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table for the account's
authentication key.

Kept as a private entry function to ensure that after an unproven rotation via
<code><a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>()</code>, the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table is only updated under the
authority of the new authentication key.


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_set_originating_address">set_originating_address</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_set_originating_address">set_originating_address</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a>, <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> {
    <b>let</b> account_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    <b>let</b> auth_key_as_address =
        <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="account.md#0x1_account_Account">Account</a>[account_addr].authentication_key);
    <b>let</b> address_map_ref_mut =
        &<b>mut</b> <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[@aptos_framework].address_map;
    <b>if</b> (address_map_ref_mut.contains(auth_key_as_address)) {
        <b>assert</b>!(
            *address_map_ref_mut.borrow(auth_key_as_address) == account_addr,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ENEW_AUTH_KEY_ALREADY_MAPPED">ENEW_AUTH_KEY_ALREADY_MAPPED</a>)
        );
    } <b>else</b> {
        address_map_ref_mut.add(auth_key_as_address, account_addr);
    };
}
</code></pre>



</details>

<a id="0x1_account_is_rotation_capability_offered"></a>

## Function `is_rotation_capability_offered`

Returns true if the account at <code>account_addr</code> has a rotation capability offer.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_rotation_capability_offered">is_rotation_capability_offered</a>(account_addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_rotation_capability_offered">is_rotation_capability_offered</a>(account_addr: <b>address</b>): bool <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        <b>if</b> (!<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(account_addr)) {
            <b>return</b> <b>false</b>;
        }
    } <b>else</b> {
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(account_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    };
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[account_addr];
    account_resource.rotation_capability_offer.for.is_some()
}
</code></pre>



</details>

<a id="0x1_account_get_rotation_capability_offer_for"></a>

## Function `get_rotation_capability_offer_for`

Returns the address of the account that has a rotation capability offer from the account at <code>account_addr</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_rotation_capability_offer_for">get_rotation_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_rotation_capability_offer_for">get_rotation_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(account_addr, <a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>);
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[account_addr];
    <b>assert</b>!(
        account_resource.rotation_capability_offer.for.is_some(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SIGNER_CAPABILITY_OFFERED">ENO_SIGNER_CAPABILITY_OFFERED</a>),
    );
    *account_resource.rotation_capability_offer.for.borrow()
}
</code></pre>



</details>

<a id="0x1_account_revoke_rotation_capability"></a>

## Function `revoke_rotation_capability`

Revoke the rotation capability offer given to <code>to_be_revoked_recipient_address</code> from <code><a href="account.md#0x1_account">account</a></code>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_rotation_capability">revoke_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_rotation_capability">revoke_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(to_be_revoked_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(addr, <a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>);
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[addr];
    <b>assert</b>!(
        account_resource.rotation_capability_offer.for.contains(&to_be_revoked_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>)
    );
    <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>);
}
</code></pre>



</details>

<a id="0x1_account_revoke_any_rotation_capability"></a>

## Function `revoke_any_rotation_capability`

Revoke any rotation capability offer in the specified account.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_check_rotation_permission">check_rotation_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> offerer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(offerer_addr, <a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>);
    <b>let</b> account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>)];
    account_resource.rotation_capability_offer.for.extract();
}
</code></pre>



</details>

<a id="0x1_account_offer_signer_capability"></a>

## Function `offer_signer_capability`

Offers signer capability on behalf of <code><a href="account.md#0x1_account">account</a></code> to the account at address <code>recipient_address</code>.
An account can delegate its signer capability to only one other address at one time.
<code>signer_capability_key_bytes</code> is the <code><a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a></code> signed by the account owner's key
<code>account_scheme</code> is the scheme of the account (ed25519 or multi_ed25519).
<code>account_public_key_bytes</code> is the public key of the account owner.
<code>recipient_address</code> is the address of the recipient of the signer capability - note that if there's an existing
<code>recipient_address</code> in the account owner's <code>SignerCapabilityOffer</code>, this will replace the
previous <code>recipient_address</code> upon successful verification (the previous recipient will no longer have access
to the account owner's signer capability).


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_signer_capability">offer_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_signer_capability">offer_signer_capability</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    signer_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    account_scheme: u8,
    account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    recipient_address: <b>address</b>
) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_check_offering_permission">check_offering_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> source_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(source_address);
    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(recipient_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));

    // Proof that this <a href="account.md#0x1_account">account</a> intends <b>to</b> delegate its <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> <b>to</b> another <a href="account.md#0x1_account">account</a>.
    <b>let</b> proof_challenge = <a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a> {
        sequence_number: <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(source_address),
        source_address,
        recipient_address,
    };
    <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>(
        source_address, account_scheme, account_public_key_bytes, signer_capability_sig_bytes, proof_challenge);

    // Update the existing <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> offer or put in a new <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> offer for the recipient.
    <b>let</b> account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[source_address];
    account_resource.signer_capability_offer.for.swap_or_fill(recipient_address);
}
</code></pre>



</details>

<a id="0x1_account_is_signer_capability_offered"></a>

## Function `is_signer_capability_offered`

Returns true if the account at <code>account_addr</code> has a signer capability offer.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_signer_capability_offered">is_signer_capability_offered</a>(account_addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_signer_capability_offered">is_signer_capability_offered</a>(account_addr: <b>address</b>): bool <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        <b>if</b> (!<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(account_addr)) {
            <b>return</b> <b>false</b>;
        }
    } <b>else</b> {
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(account_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    };
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[account_addr];
    account_resource.signer_capability_offer.for.is_some()
}
</code></pre>



</details>

<a id="0x1_account_get_signer_capability_offer_for"></a>

## Function `get_signer_capability_offer_for`

Returns the address of the account that has a signer capability offer from the account at <code>account_addr</code>.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_offer_for">get_signer_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_offer_for">get_signer_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(account_addr, <a href="account.md#0x1_account_ENO_SIGNER_CAPABILITY_OFFERED">ENO_SIGNER_CAPABILITY_OFFERED</a>);
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[account_addr];
    <b>assert</b>!(
        account_resource.signer_capability_offer.for.is_some(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SIGNER_CAPABILITY_OFFERED">ENO_SIGNER_CAPABILITY_OFFERED</a>),
    );
    *account_resource.signer_capability_offer.for.borrow()
}
</code></pre>



</details>

<a id="0x1_account_revoke_signer_capability"></a>

## Function `revoke_signer_capability`

Revoke the account owner's signer capability offer for <code>to_be_revoked_address</code> (i.e., the address that
has a signer capability offer from <code><a href="account.md#0x1_account">account</a></code> but will be revoked in this function).


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_signer_capability">revoke_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_signer_capability">revoke_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_check_offering_permission">check_offering_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(to_be_revoked_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(addr, <a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>);
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[addr];
    <b>assert</b>!(
        account_resource.signer_capability_offer.for.contains(&to_be_revoked_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>)
    );
    <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>);
}
</code></pre>



</details>

<a id="0x1_account_revoke_any_signer_capability"></a>

## Function `revoke_any_signer_capability`

Revoke any signer capability offer in the specified account.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_check_offering_permission">check_offering_permission</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> offerer_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(offerer_addr, <a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>);
    <b>let</b> account_resource = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>)];
    account_resource.signer_capability_offer.for.extract();
}
</code></pre>



</details>

<a id="0x1_account_create_authorized_signer"></a>

## Function `create_authorized_signer`

Return an authorized signer of the offerer, if there's an existing signer capability offer for <code><a href="account.md#0x1_account">account</a></code>
at the offerer's address.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_authorized_signer">create_authorized_signer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offerer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_authorized_signer">create_authorized_signer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offerer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="account.md#0x1_account_check_offering_permission">check_offering_permission</a>(<a href="account.md#0x1_account">account</a>);
    <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(offerer_address, <a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>);
    // Check <b>if</b> there's an existing <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> offer from the offerer.
    <b>let</b> account_resource = &<a href="account.md#0x1_account_Account">Account</a>[offerer_address];
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(
        account_resource.signer_capability_offer.for.contains(&addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>)
    );

    <a href="create_signer.md#0x1_create_signer">create_signer</a>(offerer_address)
}
</code></pre>



</details>

<a id="0x1_account_assert_account_resource_with_error"></a>

## Function `assert_account_resource_with_error`



<pre><code><b>fun</b> <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, error_code: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="account.md#0x1_account_assert_account_resource_with_error">assert_account_resource_with_error</a>(<a href="account.md#0x1_account">account</a>: <b>address</b>, error_code: u64) {
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_default_account_resource_enabled">features::is_default_account_resource_enabled</a>()) {
        <b>assert</b>!(
            <a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(<a href="account.md#0x1_account">account</a>),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(error_code),
        );
    } <b>else</b> {
        <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(<a href="account.md#0x1_account">account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));
    };
}
</code></pre>



</details>

<a id="0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

## Function `assert_valid_rotation_proof_signature_and_get_auth_key`

Helper functions for authentication key rotation.


<pre><code><b>fun</b> <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, challenge: &<a href="account.md#0x1_account_RotationProofChallenge">account::RotationProofChallenge</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(
    scheme: u8,
    public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    challenge: &<a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>if</b> (scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) {
        <b>let</b> pk = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(public_key_bytes);
        <b>let</b> sig = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_signature_from_bytes">ed25519::new_signature_from_bytes</a>(signature);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict_t">ed25519::signature_verify_strict_t</a>(&sig, &pk, *challenge),
            std::error::invalid_argument(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)
        );
        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&pk)
    } <b>else</b> <b>if</b> (scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) {
        <b>let</b> pk = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(public_key_bytes);
        <b>let</b> sig = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">multi_ed25519::new_signature_from_bytes</a>(signature);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">multi_ed25519::signature_verify_strict_t</a>(&sig, &pk, *challenge),
            std::error::invalid_argument(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)
        );
        <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&pk)
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)
    }
}
</code></pre>



</details>

<a id="0x1_account_update_auth_key_and_originating_address_table"></a>

## Function `update_auth_key_and_originating_address_table`

Update the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table, so that we can find the originating address using the latest address
in the event of key recovery.


<pre><code><b>fun</b> <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(originating_addr: <b>address</b>, account_resource: &<b>mut</b> <a href="account.md#0x1_account_Account">account::Account</a>, new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(
    originating_addr: <b>address</b>,
    account_resource: &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>,
    new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
) <b>acquires</b> <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> {
    <b>let</b> address_map = &<b>mut</b> <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[@aptos_framework].address_map;
    <b>let</b> curr_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(account_resource.authentication_key);
    <b>let</b> new_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(new_auth_key_vector);
    <b>assert</b>!(
        new_auth_key != curr_auth_key,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ENEW_AUTH_KEY_SAME_AS_CURRENT">ENEW_AUTH_KEY_SAME_AS_CURRENT</a>)
    );

    // Checks `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[curr_auth_key]` is either unmapped, or mapped <b>to</b> `originating_address`.
    // If it's mapped <b>to</b> the originating <b>address</b>, removes that mapping.
    // Otherwise, <b>abort</b> <b>if</b> it's mapped <b>to</b> a different <b>address</b>.
    <b>if</b> (address_map.contains(curr_auth_key)) {
        // If account_a <b>with</b> address_a is rotating its keypair from keypair_a <b>to</b> keypair_b, we expect
        // the <b>address</b> of the <a href="account.md#0x1_account">account</a> <b>to</b> stay the same, <b>while</b> its keypair updates <b>to</b> keypair_b.
        // Here, by asserting that we're calling from the <a href="account.md#0x1_account">account</a> <b>with</b> the originating <b>address</b>, we enforce
        // the standard of keeping the same <b>address</b> and updating the keypair at the contract level.
        // Without this assertion, the dapps could also <b>update</b> the <a href="account.md#0x1_account">account</a>'s <b>address</b> <b>to</b> address_b (the <b>address</b> that
        // is programmatically related <b>to</b> keypaier_b) and <b>update</b> the keypair <b>to</b> keypair_b. This causes problems
        // for interoperability because different dapps can implement this in different ways.
        // If the <a href="account.md#0x1_account">account</a> <b>with</b> <b>address</b> b calls this function <b>with</b> two valid signatures, it will <b>abort</b> at this step,
        // because <b>address</b> b is not the <a href="account.md#0x1_account">account</a>'s originating <b>address</b>.
        <b>assert</b>!(
            originating_addr == address_map.remove(curr_auth_key),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EINVALID_ORIGINATING_ADDRESS">EINVALID_ORIGINATING_ADDRESS</a>)
        );
    };

    // Set `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[new_auth_key] = originating_address`.
    <b>assert</b>!(
        !address_map.contains(new_auth_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ENEW_AUTH_KEY_ALREADY_MAPPED">ENEW_AUTH_KEY_ALREADY_MAPPED</a>)
    );
    address_map.add(new_auth_key, originating_addr);

    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="account.md#0x1_account_KeyRotation">KeyRotation</a> {
            <a href="account.md#0x1_account">account</a>: originating_addr,
            old_authentication_key: account_resource.authentication_key,
            new_authentication_key: new_auth_key_vector,
        });
    } <b>else</b> {
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a>&gt;(
            &<b>mut</b> account_resource.key_rotation_events,
            <a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a> {
                old_authentication_key: account_resource.authentication_key,
                new_authentication_key: new_auth_key_vector,
            }
        );
    };

    // Update the <a href="account.md#0x1_account">account</a> resource's authentication key.
    account_resource.authentication_key = new_auth_key_vector;
}
</code></pre>



</details>

<a id="0x1_account_create_resource_address"></a>

## Function `create_resource_address`

Basic account creation methods.
This is a helper function to compute resource addresses. Computation of the address
involves the use of a cryptographic hash operation and should be use thoughtfully.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b> {
    <b>let</b> bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(source);
    bytes.append(seed);
    bytes.push_back(<a href="account.md#0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME">DERIVE_RESOURCE_ACCOUNT_SCHEME</a>);
    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes))
}
</code></pre>



</details>

<a id="0x1_account_create_resource_account"></a>

## Function `create_resource_account`

A resource account is used to manage resources independent of an account managed by a user.
In Aptos a resource account is created based upon the sha3 256 of the source's address and additional seed data.
A resource account can only be created once, this is designated by setting the
<code>Account::signer_capability_offer::for</code> to the address of the resource account. While an entity may call
<code>create_account</code> to attempt to claim an account ahead of the creation of a resource account, if found Aptos will
transition ownership of the account over to the resource account. This is done by validating that the account has
yet to execute any transactions and that the <code>Account::signer_capability_offer::for</code> is none. The probability of a
collision where someone has legitimately produced a private key that maps to a resource account address is less
than <code>(1/2)^(256)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_account">create_resource_account</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_account">create_resource_account</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">SignerCapability</a>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>let</b> resource_addr = <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source), seed);
    <b>let</b> resource = <b>if</b> (<a href="account.md#0x1_account_exists_at">exists_at</a>(resource_addr)) {
        <b>if</b> (<a href="account.md#0x1_account_resource_exists_at">resource_exists_at</a>(resource_addr)) {
        <b>let</b> <a href="account.md#0x1_account">account</a> = &<a href="account.md#0x1_account_Account">Account</a>[resource_addr];
        <b>assert</b>!(
            <a href="account.md#0x1_account">account</a>.signer_capability_offer.for.is_none(),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="account.md#0x1_account_ERESOURCE_ACCCOUNT_EXISTS">ERESOURCE_ACCCOUNT_EXISTS</a>),
        );
        };
        <b>assert</b>!(
            <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(resource_addr) == 0,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="account.md#0x1_account_EACCOUNT_ALREADY_USED">EACCOUNT_ALREADY_USED</a>),
        );
        <a href="create_signer.md#0x1_create_signer">create_signer</a>(resource_addr)
    } <b>else</b> {
        <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(resource_addr)
    };

    // By default, only the <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> should have control over the resource <a href="account.md#0x1_account">account</a> and not the auth key.
    // If the source <a href="account.md#0x1_account">account</a> wants direct control via auth key, they would need <b>to</b> explicitly rotate the auth key
    // of the resource <a href="account.md#0x1_account">account</a> using the <a href="account.md#0x1_account_SignerCapability">SignerCapability</a>.
    <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(&resource, <a href="account.md#0x1_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>);

    <b>let</b> <a href="account.md#0x1_account">account</a> = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[resource_addr];
    <a href="account.md#0x1_account">account</a>.signer_capability_offer.for = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(resource_addr);
    <b>let</b> signer_cap = <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> { <a href="account.md#0x1_account">account</a>: resource_addr };
    (resource, signer_cap)
}
</code></pre>



</details>

<a id="0x1_account_create_framework_reserved_account"></a>

## Function `create_framework_reserved_account`

create the account for system reserved addresses


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_framework_reserved_account">create_framework_reserved_account</a>(addr: <b>address</b>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_framework_reserved_account">create_framework_reserved_account</a>(addr: <b>address</b>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">SignerCapability</a>) {
    <b>assert</b>!(
        addr == @0x1 ||
            addr == @0x2 ||
            addr == @0x3 ||
            addr == @0x4 ||
            addr == @0x5 ||
            addr == @0x6 ||
            addr == @0x7 ||
            addr == @0x8 ||
            addr == @0x9 ||
            addr == @0xa ||
            addr == @0xb,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account.md#0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS">ENO_VALID_FRAMEWORK_RESERVED_ADDRESS</a>),
    );
    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> = <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(addr);
    <b>let</b> signer_cap = <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> { <a href="account.md#0x1_account">account</a>: addr };
    (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_cap)
}
</code></pre>



</details>

<a id="0x1_account_create_guid"></a>

## Function `create_guid`

GUID management methods.
Creates a new GUID for <code>account_signer</code> and increments the GUID creation number.

When the <code>default_account_resource</code> feature flag is enabled:
- If no Account resource exists, one will be created automatically
- This ensures consistent GUID creation behavior for all addresses

When the feature flag is disabled:
- Aborts if no Account resource exists

Aborts if the maximum number of GUIDs has been reached (0x4000000000000)


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_guid">create_guid</a>(account_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_guid">create_guid</a>(account_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(account_signer);
    <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(addr);
    <b>let</b> <a href="account.md#0x1_account">account</a> = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[addr];
    <b>let</b> <a href="guid.md#0x1_guid">guid</a> = <a href="guid.md#0x1_guid_create">guid::create</a>(addr, &<b>mut</b> <a href="account.md#0x1_account">account</a>.guid_creation_num);
    <b>assert</b>!(
        <a href="account.md#0x1_account">account</a>.guid_creation_num &lt; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">MAX_GUID_CREATION_NUM</a>,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="account.md#0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM">EEXCEEDED_MAX_GUID_CREATION_NUM</a>),
    );
    <a href="guid.md#0x1_guid">guid</a>
}
</code></pre>



</details>

<a id="0x1_account_new_event_handle"></a>

## Function `new_event_handle`

Creates a new event handle for <code><a href="account.md#0x1_account">account</a></code>.

This is a wrapper around <code>create_guid</code> that creates an EventHandle,
inheriting the same behavior regarding account existence and feature flags.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_new_event_handle">new_event_handle</a>&lt;T: drop + store&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): EventHandle&lt;T&gt; <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>(<a href="account.md#0x1_account_create_guid">create_guid</a>(<a href="account.md#0x1_account">account</a>))
}
</code></pre>



</details>

<a id="0x1_account_register_coin"></a>

## Function `register_coin`

Coin management methods.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_register_coin">register_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_register_coin">register_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>if</b> (std::features::module_event_migration_enabled()) {
        <a href="event.md#0x1_event_emit">event::emit</a>(
            <a href="account.md#0x1_account_CoinRegister">CoinRegister</a> {
                <a href="account.md#0x1_account">account</a>: account_addr,
                <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">type_info</a>: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;(),
            },
        );
    } <b>else</b> {
        <a href="account.md#0x1_account_ensure_resource_exists">ensure_resource_exists</a>(account_addr);
        <b>let</b> <a href="account.md#0x1_account">account</a> = &<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>[account_addr];
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a>&gt;(
            &<b>mut</b> <a href="account.md#0x1_account">account</a>.coin_register_events,
            <a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a> {
                <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">type_info</a>: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;(),
            },
        );
    }
}
</code></pre>



</details>

<a id="0x1_account_create_signer_with_capability"></a>

## Function `create_signer_with_capability`

Capability based functions for efficient use.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_signer_with_capability">create_signer_with_capability</a>(<a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>: &<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_signer_with_capability">create_signer_with_capability</a>(<a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>: &<a href="account.md#0x1_account_SignerCapability">SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> addr = &<a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>.<a href="account.md#0x1_account">account</a>;
    <a href="create_signer.md#0x1_create_signer">create_signer</a>(*addr)
}
</code></pre>



</details>

<a id="0x1_account_get_signer_capability_address"></a>

## Function `get_signer_capability_address`



<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_address">get_signer_capability_address</a>(<a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>: &<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_address">get_signer_capability_address</a>(<a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>: &<a href="account.md#0x1_account_SignerCapability">SignerCapability</a>): <b>address</b> {
    <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>.<a href="account.md#0x1_account">account</a>
}
</code></pre>



</details>

<a id="0x1_account_verify_signed_message"></a>

## Function `verify_signed_message`



<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>&lt;T: drop&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, account_scheme: u8, account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signed_message_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>&lt;T: drop&gt;(
    <a href="account.md#0x1_account">account</a>: <b>address</b>,
    account_scheme: u8,
    account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    signed_message_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    message: T,
) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> {
    <b>let</b> auth_key = <a href="account.md#0x1_account_get_authentication_key">get_authentication_key</a>(<a href="account.md#0x1_account">account</a>);
    // Verify that the `<a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a>` <b>has</b> the right information and is signed by the <a href="account.md#0x1_account">account</a> owner's key
    <b>if</b> (account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) {
        <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key);
        <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&pubkey);
        <b>assert</b>!(
            auth_key == expected_auth_key,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>),
        );

        <b>let</b> signer_capability_sig = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_signature_from_bytes">ed25519::new_signature_from_bytes</a>(signed_message_bytes);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict_t">ed25519::signature_verify_strict_t</a>(&signer_capability_sig, &pubkey, message),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>),
        );
    } <b>else</b> <b>if</b> (account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) {
        <b>let</b> pubkey = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key);
        <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&pubkey);
        <b>assert</b>!(
            auth_key == expected_auth_key,
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>),
        );

        <b>let</b> signer_capability_sig = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">multi_ed25519::new_signature_from_bytes</a>(signed_message_bytes);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">multi_ed25519::signature_verify_strict_t</a>(&signer_capability_sig, &pubkey, message),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>),
        );
    } <b>else</b> {
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)
    };
}
</code></pre>



</details>

<a id="@Specification_4"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The initialization of the account module should result in the proper system initialization with valid and consistent resources.</td>
<td>High</td>
<td>Initialization of the account module creates a valid address_map table and moves the resources to the OriginatingAddress under the aptos_framework account.</td>
<td>Audited that the address_map table is created and populated correctly with the expected initial values.</td>
</tr>

<tr>
<td>2</td>
<td>After successfully creating an account, the account resources should initialize with the default data, ensuring the proper initialization of the account state.</td>
<td>High</td>
<td>Creating an account via the create_account function validates the state and moves a new account resource under new_address.</td>
<td>Formally verified via <a href="#high-level-req-2">create_account</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Checking the existence of an account under a given address never results in an abort.</td>
<td>Low</td>
<td>The exists_at function returns a boolean value indicating the existence of an account under the given address.</td>
<td>Formally verified by the <a href="#high-level-req-3">aborts_if</a> condition.</td>
</tr>

<tr>
<td>4</td>
<td>The account module maintains bounded sequence numbers for all accounts, guaranteeing they remain within the specified limit.</td>
<td>Medium</td>
<td>The sequence number of an account may only increase up to MAX_U64 in a succeeding manner.</td>
<td>Formally verified via <a href="#high-level-req-4">increment_sequence_number</a> that it remains within the defined boundary of MAX_U64.</td>
</tr>

<tr>
<td>5</td>
<td>Only the ed25519 and multied25519 signature schemes are permissible.</td>
<td>Low</td>
<td>Exclusively perform key rotation using either the ed25519 or multied25519 signature schemes. Currently restricts the offering of rotation/signing capabilities to the ed25519 or multied25519 schemes.</td>
<td>Formally Verified: <a href="#high-level-req-5.1">rotate_authentication_key</a>, <a href="#high-level-req-5.2">offer_rotation_capability</a>, and <a href="#high-level-req-5.3">offer_signer_capability</a>. Verified that it aborts if the account_scheme is not ED25519_SCHEME and not MULTI_ED25519_SCHEME. Audited that the scheme enums correspond correctly to signature logic.</td>
</tr>

<tr>
<td>6</td>
<td>Exclusively permit the rotation of the authentication key of an account for the account owner or any user who possesses rotation capabilities associated with that account.</td>
<td>Critical</td>
<td>In the rotate_authentication_key function, the authentication key derived from the from_public_key_bytes should match the signer's current authentication key. Only the delegate_signer granted the rotation capabilities may invoke the rotate_authentication_key_with_rotation_capability function.</td>
<td>Formally Verified via <a href="#high-level-req-6.1">rotate_authentication_key</a> and <a href="#high-level-req-6.2">rotate_authentication_key_with_rotation_capability</a>.</td>
</tr>

<tr>
<td>7</td>
<td>Only the owner of an account may offer or revoke the following capabilities: (1) offer_rotation_capability, (2) offer_signer_capability, (3) revoke_rotation_capability, and (4) revoke_signer_capability.</td>
<td>Critical</td>
<td>An account resource may only be modified by the owner of the account utilizing: rotation_capability_offer, signer_capability_offer.</td>
<td>Formally verified via <a href="#high-level-req-7.1">offer_rotation_capability</a>, <a href="#high-level-req-7.2">offer_signer_capability</a>, and <a href="#high-level-req-7.3">revoke_rotation_capability</a>. and <a href="#high-level-req-7.4">revoke_signer_capability</a>.</td>
</tr>

<tr>
<td>8</td>
<td>The capability to create a signer for the account is exclusively reserved for either the account owner or the account that has been granted the signing capabilities.</td>
<td>Critical</td>
<td>Signer creation for the account may only be successfully executed by explicitly granting the signing capabilities with the create_authorized_signer function.</td>
<td>Formally verified via <a href="#high-level-req-8">create_authorized_signer</a>.</td>
</tr>

<tr>
<td>9</td>
<td>Rotating the authentication key requires two valid signatures. With the private key of the current authentication key. With the private key of the new authentication key.</td>
<td>Critical</td>
<td>The rotate_authentication_key verifies two signatures (current and new) before rotating to the new key. The first signature ensures the user has the intended capability, and the second signature ensures that the user owns the new key.</td>
<td>Formally verified via <a href="#high-level-req-9.1">rotate_authentication_key</a> and <a href="#high-level-req-9.2">rotate_authentication_key_with_rotation_capability</a>.</td>
</tr>

<tr>
<td>10</td>
<td>The rotation of the authentication key updates the account's authentication key with the newly supplied one.</td>
<td>High</td>
<td>The auth_key may only update to the provided new_auth_key after verifying the signature.</td>
<td>Formally Verified in <a href="#high-level-req-10">rotate_authentication_key_internal</a> that the authentication key of an account is modified to the provided authentication key if the signature verification was successful.</td>
</tr>

<tr>
<td>11</td>
<td>The creation number is monotonically increasing.</td>
<td>Low</td>
<td>The guid_creation_num in the Account structure is monotonically increasing.</td>
<td>Formally Verified via <a href="#high-level-req-11">guid_creation_num</a>.</td>
</tr>

<tr>
<td>12</td>
<td>The Account resource is persistent.</td>
<td>Low</td>
<td>The Account structure assigned to the address should be persistent.</td>
<td>Audited that the Account structure is persistent.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>



<a id="@Specification_4_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>


Only the address <code>@aptos_framework</code> can call.
OriginatingAddress does not exist under <code>@aptos_framework</code> before the call.


<pre><code><b>let</b> aptos_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(aptos_addr);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(aptos_addr);
</code></pre>



<a id="@Specification_4_create_account_if_does_not_exist"></a>

### Function `create_account_if_does_not_exist`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
</code></pre>


Ensure that the account exists at the end of the call.


<pre><code><b>let</b> authentication_key = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(account_address);
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address) && (
    account_address == @vm_reserved
    || account_address == @aptos_framework
    || account_address == @aptos_token
    || !(len(authentication_key) == 32)
);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address);
</code></pre>



<a id="@Specification_4_create_account"></a>

### Function `create_account`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.
Limit the new account address is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code><b>include</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> {addr: new_address};
<b>aborts_if</b> new_address == @vm_reserved || new_address == @aptos_framework || new_address == @aptos_token;
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) == new_address;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(new_address);
</code></pre>



<a id="@Specification_4_create_account_unchecked"></a>

### Function `create_account_unchecked`


<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.


<pre><code><b>pragma</b> opaque;
<b>include</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> {addr: new_address};
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(new_address);
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) == new_address;
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(new_address);
</code></pre>



<a id="@Specification_4_exists_at"></a>

### Function `exists_at`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_exists_at">exists_at</a>(addr: <b>address</b>): bool
</code></pre>




<pre><code><b>pragma</b> opaque;
// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
<b>aborts_if</b> <b>false</b>;
<b>ensures</b> result == <a href="account.md#0x1_account_spec_exists_at">spec_exists_at</a>(addr);
</code></pre>




<a id="0x1_account_spec_exists_at"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_exists_at">spec_exists_at</a>(addr: <b>address</b>): bool {
   <b>use</b> std::features;
   <b>use</b> std::features::DEFAULT_ACCOUNT_RESOURCE;
   <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_is_enabled">features::spec_is_enabled</a>(DEFAULT_ACCOUNT_RESOURCE) || <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr)
}
</code></pre>




<a id="0x1_account_CreateAccountAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> {
    addr: <b>address</b>;
    <b>let</b> authentication_key = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(addr);
    <b>aborts_if</b> len(authentication_key) != 32;
    <b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
    <b>ensures</b> len(authentication_key) == 32;
}
</code></pre>



<a id="@Specification_4_get_guid_next_creation_num"></a>

### Function `get_guid_next_creation_num`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_guid_next_creation_num">get_guid_next_creation_num</a>(addr: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>ensures</b> result == <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).guid_creation_num;
</code></pre>



<a id="@Specification_4_get_sequence_number"></a>

### Function `get_sequence_number`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>ensures</b> result == <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number;
</code></pre>



<a id="@Specification_4_originating_address"></a>

### Function `originating_address`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_originating_address">originating_address</a>(auth_key: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<b>address</b>&gt;
</code></pre>




<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>



<a id="@Specification_4_increment_sequence_number"></a>

### Function `increment_sequence_number`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>)
</code></pre>


The Account existed under the address.
The sequence_number of the Account is up to MAX_U64.


<pre><code><b>let</b> sequence_number = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
<b>aborts_if</b> sequence_number == <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>;
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>let</b> <b>post</b> post_sequence_number = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number;
<b>ensures</b> post_sequence_number == sequence_number + 1;
</code></pre>



<a id="@Specification_4_get_authentication_key"></a>

### Function `get_authentication_key`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_authentication_key">get_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>ensures</b> result == <a href="account.md#0x1_account_spec_get_authentication_key">spec_get_authentication_key</a>(addr);
</code></pre>




<a id="0x1_account_spec_get_authentication_key"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_get_authentication_key">spec_get_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
   <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).authentication_key
}
</code></pre>



<a id="@Specification_4_rotate_authentication_key_internal"></a>

### Function `rotate_authentication_key_internal`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


The Account existed under the signer before the call.
The length of new_auth_key is 32.


<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
// This enforces <a id="high-level-req-10" href="#high-level-req">high-level requirement 10</a>:
<b>let</b> <b>post</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(new_auth_key) != 32;
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>ensures</b> account_resource.authentication_key == new_auth_key;
</code></pre>



<a id="@Specification_4_rotate_authentication_key_call"></a>

### Function `rotate_authentication_key_call`


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
// This enforces <a id="high-level-req-10" href="#high-level-req">high-level requirement 10</a>:
<b>let</b> <b>post</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(new_auth_key) != 32;
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>ensures</b> account_resource.authentication_key == new_auth_key;
</code></pre>



<a id="@Specification_4_rotate_authentication_key_from_public_key"></a>

### Function `rotate_authentication_key_from_public_key`


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_from_public_key">rotate_authentication_key_from_public_key</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, scheme: u8, new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>aborts_if</b> scheme != <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && scheme != <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && scheme != <a href="account.md#0x1_account_SINGLE_KEY_SCHEME">SINGLE_KEY_SCHEME</a> && scheme != <a href="account.md#0x1_account_MULTI_KEY_SCHEME">MULTI_KEY_SCHEME</a>;
</code></pre>




<a id="0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, challenge: <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



<a id="@Specification_4_rotate_authentication_key"></a>

### Function `rotate_authentication_key`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key">rotate_authentication_key</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from_scheme: u8, from_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_rotate_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


The Account existed under the signer
The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME


<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
// This enforces <a id="high-level-req-6.1" href="#high-level-req">high-level requirement 6</a>:
<b>include</b> from_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: from_public_key_bytes };
<b>aborts_if</b> from_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && ({
    <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(from_public_key_bytes);
    account_resource.authentication_key != expected_auth_key
});
<b>include</b> from_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: from_public_key_bytes };
<b>aborts_if</b> from_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && ({
    <b>let</b> from_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(from_public_key_bytes);
    account_resource.authentication_key != from_auth_key
});
// This enforces <a id="high-level-req-5.1" href="#high-level-req">high-level requirement 5</a>:
<b>aborts_if</b> from_scheme != <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && from_scheme != <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;
<b>let</b> curr_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);
<b>let</b> challenge = <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> {
    sequence_number: account_resource.sequence_number,
    originator: addr,
    current_auth_key: curr_auth_key,
    new_public_key: to_public_key_bytes,
};
// This enforces <a id="high-level-req-9.1" href="#high-level-req">high-level requirement 9</a>:
<b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> {
    scheme: from_scheme,
    public_key_bytes: from_public_key_bytes,
    signature: cap_rotate_key,
    challenge,
};
<b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> {
    scheme: to_scheme,
    public_key_bytes: to_public_key_bytes,
    signature: cap_update_table,
    challenge,
};
<b>let</b> originating_addr = addr;
<b>let</b> new_auth_key_vector = <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(to_scheme, to_public_key_bytes, cap_update_table, challenge);
<b>let</b> address_map = <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map;
<b>let</b> new_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, curr_auth_key) &&
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(address_map, curr_auth_key) != originating_addr;
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(new_auth_key_vector);
<b>aborts_if</b> curr_auth_key != new_auth_key && <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, new_auth_key);
<b>include</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a> {
    originating_addr: addr,
};
<b>let</b> <b>post</b> auth_key = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).authentication_key;
<b>ensures</b> auth_key == new_auth_key_vector;
</code></pre>



<a id="@Specification_4_rotate_authentication_key_with_rotation_capability"></a>

### Function `rotate_authentication_key_with_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_with_rotation_capability">rotate_authentication_key_with_rotation_capability</a>(delegate_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_cap_offerer_address: <b>address</b>, new_scheme: u8, new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address);
<b>let</b> delegate_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegate_signer);
<b>let</b> offerer_account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(offerer_account_resource.authentication_key);
<b>let</b> curr_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(offerer_account_resource.authentication_key);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(delegate_address);
<b>let</b> challenge = <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> {
    sequence_number: <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(delegate_address).sequence_number,
    originator: rotation_cap_offerer_address,
    current_auth_key: curr_auth_key,
    new_public_key: new_public_key_bytes,
};
// This enforces <a id="high-level-req-6.2" href="#high-level-req">high-level requirement 6</a>:
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(offerer_account_resource.rotation_capability_offer.for, delegate_address);
// This enforces <a id="high-level-req-9.1" href="#high-level-req">high-level requirement 9</a>:
<b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> {
    scheme: new_scheme,
    public_key_bytes: new_public_key_bytes,
    signature: cap_update_table,
    challenge,
};
<b>let</b> new_auth_key_vector = <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(new_scheme, new_public_key_bytes, cap_update_table, challenge);
<b>let</b> address_map = <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(offerer_account_resource.authentication_key);
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, curr_auth_key) &&
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(address_map, curr_auth_key) != rotation_cap_offerer_address;
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(new_auth_key_vector);
<b>let</b> new_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector);
<b>aborts_if</b> curr_auth_key != new_auth_key && <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, new_auth_key);
<b>include</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a> {
    originating_addr: rotation_cap_offerer_address,
    account_resource: offerer_account_resource,
};
<b>let</b> <b>post</b> auth_key = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address).authentication_key;
<b>ensures</b> auth_key == new_auth_key_vector;
</code></pre>



<a id="@Specification_4_offer_rotation_capability"></a>

### Function `offer_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_rotation_capability">offer_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)
</code></pre>




<pre><code><b>let</b> source_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);
<b>let</b> proof_challenge = <a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a> {
    <a href="chain_id.md#0x1_chain_id">chain_id</a>: <b>global</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">chain_id::ChainId</a>&gt;(@aptos_framework).id,
    sequence_number: account_resource.sequence_number,
    source_address,
    recipient_address,
};
<b>aborts_if</b> !<b>exists</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">chain_id::ChainId</a>&gt;(@aptos_framework);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(recipient_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);
<b>include</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: account_public_key_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && ({
    <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);
    account_resource.authentication_key != expected_auth_key
});
<b>include</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: rotation_capability_sig_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && !<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_signature_verify_strict_t">ed25519::spec_signature_verify_strict_t</a>(
    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_Signature">ed25519::Signature</a> { bytes: rotation_capability_sig_bytes },
    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a> { bytes: account_public_key_bytes },
    proof_challenge
);
<b>include</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: account_public_key_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && ({
    <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);
    account_resource.authentication_key != expected_auth_key
});
<b>include</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: rotation_capability_sig_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && !<a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_t">multi_ed25519::spec_signature_verify_strict_t</a>(
    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a> { bytes: rotation_capability_sig_bytes },
    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a> { bytes: account_public_key_bytes },
    proof_challenge
);
// This enforces <a id="high-level-req-5.2" href="#high-level-req">high-level requirement 5</a>:
<b>aborts_if</b> account_scheme != <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && account_scheme != <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;
// This enforces <a id="high-level-req-7.1" href="#high-level-req">high-level requirement 7</a>:
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);
<b>let</b> <b>post</b> offer_for = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address).rotation_capability_offer.for;
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(offer_for) == recipient_address;
</code></pre>



<a id="@Specification_4_set_originating_address"></a>

### Function `set_originating_address`


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_set_originating_address">set_originating_address</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>



<a id="@Specification_4_is_rotation_capability_offered"></a>

### Function `is_rotation_capability_offered`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_rotation_capability_offered">is_rotation_capability_offered</a>(account_addr: <b>address</b>): bool
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
</code></pre>



<a id="@Specification_4_get_rotation_capability_offer_for"></a>

### Function `get_rotation_capability_offer_for`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_rotation_capability_offer_for">get_rotation_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
<b>aborts_if</b> len(account_resource.rotation_capability_offer.for.vec) == 0;
</code></pre>



<a id="@Specification_4_revoke_rotation_capability"></a>

### Function `revoke_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_rotation_capability">revoke_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(account_resource.rotation_capability_offer.for,to_be_revoked_address);
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);
<b>let</b> <b>post</b> offer_for = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).rotation_capability_offer.for;
<b>ensures</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(offer_for);
</code></pre>



<a id="@Specification_4_revoke_any_rotation_capability"></a>

### Function `revoke_any_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
// This enforces <a id="high-level-req-7.3" href="#high-level-req">high-level requirement 7</a>:
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(account_resource.rotation_capability_offer.for);
<b>let</b> <b>post</b> offer_for = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).rotation_capability_offer.for;
<b>ensures</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(offer_for);
</code></pre>



<a id="@Specification_4_offer_signer_capability"></a>

### Function `offer_signer_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_signer_capability">offer_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)
</code></pre>


The Account existed under the signer.
The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME.


<pre><code><b>let</b> source_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);
<b>let</b> proof_challenge = <a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a> {
    sequence_number: account_resource.sequence_number,
    source_address,
    recipient_address,
};
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(recipient_address);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);
<b>include</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: account_public_key_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && ({
    <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);
    account_resource.authentication_key != expected_auth_key
});
<b>include</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: signer_capability_sig_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && !<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_signature_verify_strict_t">ed25519::spec_signature_verify_strict_t</a>(
    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_Signature">ed25519::Signature</a> { bytes: signer_capability_sig_bytes },
    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a> { bytes: account_public_key_bytes },
    proof_challenge
);
<b>include</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: account_public_key_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && ({
    <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);
    account_resource.authentication_key != expected_auth_key
});
<b>include</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: signer_capability_sig_bytes };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && !<a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_t">multi_ed25519::spec_signature_verify_strict_t</a>(
    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a> { bytes: signer_capability_sig_bytes },
    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a> { bytes: account_public_key_bytes },
    proof_challenge
);
// This enforces <a id="high-level-req-5.3" href="#high-level-req">high-level requirement 5</a>:
<b>aborts_if</b> account_scheme != <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && account_scheme != <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;
// This enforces <a id="high-level-req-7.2" href="#high-level-req">high-level requirement 7</a>:
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);
<b>let</b> <b>post</b> offer_for = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address).signer_capability_offer.for;
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(offer_for) == recipient_address;
</code></pre>



<a id="@Specification_4_is_signer_capability_offered"></a>

### Function `is_signer_capability_offered`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_signer_capability_offered">is_signer_capability_offered</a>(account_addr: <b>address</b>): bool
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
</code></pre>



<a id="@Specification_4_get_signer_capability_offer_for"></a>

### Function `get_signer_capability_offer_for`


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_offer_for">get_signer_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b>
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
<b>aborts_if</b> len(account_resource.signer_capability_offer.for.vec) == 0;
</code></pre>



<a id="@Specification_4_revoke_signer_capability"></a>

### Function `revoke_signer_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_signer_capability">revoke_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)
</code></pre>


The Account existed under the signer.
The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address.


<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);
<b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(account_resource.signer_capability_offer.for,to_be_revoked_address);
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);
</code></pre>



<a id="@Specification_4_revoke_any_signer_capability"></a>

### Function `revoke_any_signer_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));
// This enforces <a id="high-level-req-7.4" href="#high-level-req">high-level requirement 7</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(account_resource.signer_capability_offer.for);
</code></pre>



<a id="@Specification_4_create_authorized_signer"></a>

### Function `create_authorized_signer`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_authorized_signer">create_authorized_signer</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offerer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>


The Account existed under the signer.
The value of signer_capability_offer.for of Account resource under the signer is offerer_address.


<pre><code>// This enforces <a id="high-level-req-8" href="#high-level-req">high-level requirement 8</a>:
<b>include</b> <a href="account.md#0x1_account_AccountContainsAddr">AccountContainsAddr</a>{
    <a href="account.md#0x1_account">account</a>,
    <b>address</b>: offerer_address,
};
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(offerer_address);
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(offerer_address);
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) == offerer_address;
</code></pre>




<a id="0x1_account_AccountContainsAddr"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_AccountContainsAddr">AccountContainsAddr</a> {
    <a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>address</b>: <b>address</b>;
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<b>address</b>);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<b>address</b>);
    // This enforces <a id="high-level-spec-3" href="create_signer.md#high-level-req">high-level requirement 3</a> of the <a href="create_signer.md">create_signer</a> module:
    <b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(account_resource.signer_capability_offer.for,addr);
}
</code></pre>



<a id="@Specification_4_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

### Function `assert_valid_rotation_proof_signature_and_get_auth_key`


<pre><code><b>fun</b> <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, challenge: &<a href="account.md#0x1_account_RotationProofChallenge">account::RotationProofChallenge</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>




<pre><code><b>pragma</b> opaque;
<b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a>;
<b>ensures</b> [abstract] result == <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme, public_key_bytes, signature, challenge);
</code></pre>




<a id="0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> {
    scheme: u8;
    public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    challenge: <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>;
    <b>include</b> scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: public_key_bytes };
    <b>include</b> scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: signature };
    <b>aborts_if</b> scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && !<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_signature_verify_strict_t">ed25519::spec_signature_verify_strict_t</a>(
        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_Signature">ed25519::Signature</a> { bytes: signature },
        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a> { bytes: public_key_bytes },
        challenge
    );
    <b>include</b> scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: public_key_bytes };
    <b>include</b> scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: signature };
    <b>aborts_if</b> scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && !<a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_t">multi_ed25519::spec_signature_verify_strict_t</a>(
        <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a> { bytes: signature },
        <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a> { bytes: public_key_bytes },
        challenge
    );
    <b>aborts_if</b> scheme != <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && scheme != <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;
}
</code></pre>



<a id="@Specification_4_update_auth_key_and_originating_address_table"></a>

### Function `update_auth_key_and_originating_address_table`


<pre><code><b>fun</b> <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(originating_addr: <b>address</b>, account_resource: &<b>mut</b> <a href="account.md#0x1_account_Account">account::Account</a>, new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);
<b>include</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a>;
</code></pre>




<a id="0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a> {
    originating_addr: <b>address</b>;
    account_resource: <a href="account.md#0x1_account_Account">Account</a>;
    new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    <b>let</b> address_map = <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map;
    <b>let</b> curr_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);
    <b>let</b> new_auth_key = <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);
    <b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);
    <b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, curr_auth_key) &&
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(address_map, curr_auth_key) != originating_addr;
    <b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(new_auth_key_vector);
    <b>aborts_if</b> curr_auth_key == new_auth_key;
    <b>aborts_if</b> curr_auth_key != new_auth_key && <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, new_auth_key);
    <b>ensures</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(<b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map, <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector));
}
</code></pre>



<a id="@Specification_4_create_resource_address"></a>

### Function `create_resource_address`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(source: &<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>
</code></pre>


The Account existed under the signer
The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address


<pre><code><b>pragma</b> opaque;
<b>pragma</b> aborts_if_is_strict = <b>false</b>;
<b>aborts_if</b> [abstract] <b>false</b>;
<b>ensures</b> [abstract] result == <a href="account.md#0x1_account_spec_create_resource_address">spec_create_resource_address</a>(source, seed);
<b>ensures</b> [abstract] source != result;
</code></pre>




<a id="0x1_account_spec_create_resource_address"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_create_resource_address">spec_create_resource_address</a>(source: <b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>;
</code></pre>



<a id="@Specification_4_create_resource_account"></a>

### Function `create_resource_account`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_account">create_resource_account</a>(source: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>




<pre><code><b>let</b> source_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);
<b>let</b> resource_addr = <a href="account.md#0x1_account_spec_create_resource_address">spec_create_resource_address</a>(source_addr, seed);
<b>aborts_if</b> len(<a href="account.md#0x1_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>) != 32;
<b>include</b> <a href="account.md#0x1_account_spec_exists_at">spec_exists_at</a>(resource_addr) ==&gt; <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">CreateResourceAccountAbortsIf</a>;
<b>include</b> !<a href="account.md#0x1_account_spec_exists_at">spec_exists_at</a>(resource_addr) ==&gt; <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> {addr: resource_addr};
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result_1) == resource_addr;
<b>let</b> <b>post</b> offer_for = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(resource_addr).signer_capability_offer.for;
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(offer_for) == resource_addr;
<b>ensures</b> result_2 == <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> { <a href="account.md#0x1_account">account</a>: resource_addr };
</code></pre>



<a id="@Specification_4_create_framework_reserved_account"></a>

### Function `create_framework_reserved_account`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_framework_reserved_account">create_framework_reserved_account</a>(addr: <b>address</b>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)
</code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.
The system reserved addresses is @0x1 / @0x2 / @0x3 / @0x4 / @0x5  / @0x6 / @0x7 / @0x8 / @0x9 / @0xa.


<pre><code><b>aborts_if</b> <a href="account.md#0x1_account_spec_is_framework_address">spec_is_framework_address</a>(addr);
<b>include</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> {addr};
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result_1) == addr;
<b>ensures</b> result_2 == <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> { <a href="account.md#0x1_account">account</a>: addr };
</code></pre>




<a id="0x1_account_spec_is_framework_address"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_is_framework_address">spec_is_framework_address</a>(addr: <b>address</b>): bool{
   addr != @0x1 &&
   addr != @0x2 &&
   addr != @0x3 &&
   addr != @0x4 &&
   addr != @0x5 &&
   addr != @0x6 &&
   addr != @0x7 &&
   addr != @0x8 &&
   addr != @0x9 &&
   addr != @0xa
}
</code></pre>



<a id="@Specification_4_create_guid"></a>

### Function `create_guid`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_guid">create_guid</a>(account_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a>
</code></pre>


The Account existed under the signer.
The guid_creation_num of the account resource is up to MAX_U64.


<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(account_signer);
<b>include</b> <a href="account.md#0x1_account_NewEventHandleAbortsIf">NewEventHandleAbortsIf</a> {
    <a href="account.md#0x1_account">account</a>: account_signer,
};
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
// This enforces <a id="high-level-req-11" href="#high-level-req">high-level requirement 11</a>:
<b>ensures</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).guid_creation_num == <b>old</b>(<b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).guid_creation_num) + 1;
</code></pre>



<a id="@Specification_4_new_event_handle"></a>

### Function `new_event_handle`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;
</code></pre>


The Account existed under the signer.
The guid_creation_num of the Account is up to MAX_U64.


<pre><code><b>include</b> <a href="account.md#0x1_account_NewEventHandleAbortsIf">NewEventHandleAbortsIf</a>;
</code></pre>




<a id="0x1_account_NewEventHandleAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_NewEventHandleAbortsIf">NewEventHandleAbortsIf</a> {
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);
    <b>let</b> <a href="account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);
    <b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num + 1 &gt; <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>;
    <b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num + 1 &gt;= <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">MAX_GUID_CREATION_NUM</a>;
}
</code></pre>



<a id="@Specification_4_register_coin"></a>

### Function `register_coin`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_register_coin">register_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
<b>aborts_if</b> !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;CoinType&gt;();
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);
</code></pre>



<a id="@Specification_4_create_signer_with_capability"></a>

### Function `create_signer_with_capability`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_signer_with_capability">create_signer_with_capability</a>(<a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>: &<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>




<pre><code><b>let</b> addr = <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a>.<a href="account.md#0x1_account">account</a>;
<b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) == addr;
</code></pre>




<a id="0x1_account_CreateResourceAccountAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">CreateResourceAccountAbortsIf</a> {
    resource_addr: <b>address</b>;
    <b>let</b> <a href="account.md#0x1_account">account</a> = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(resource_addr);
}
</code></pre>



<a id="@Specification_4_verify_signed_message"></a>

### Function `verify_signed_message`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>&lt;T: drop&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, account_scheme: u8, account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signed_message_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: T)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>);
<b>let</b> account_resource = <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>);
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>);
<b>include</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: account_public_key };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && ({
    <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key);
    account_resource.authentication_key != expected_auth_key
});
<b>include</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> { bytes: account_public_key };
<b>aborts_if</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> && ({
    <b>let</b> expected_auth_key = <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key);
    account_resource.authentication_key != expected_auth_key
});
<b>include</b> account_scheme == <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: signed_message_bytes };
<b>include</b> account_scheme == <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> ==&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> { bytes: signed_message_bytes };
<b>aborts_if</b> account_scheme != <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> && account_scheme != <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
