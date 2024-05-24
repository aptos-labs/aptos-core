
<a id="0x1_account"></a>

# Module `0x1::account`



-  [Struct `KeyRotation`](#0x1_account_KeyRotation)
-  [Resource `Account`](#0x1_account_Account)
-  [Struct `KeyRotationEvent`](#0x1_account_KeyRotationEvent)
-  [Struct `CoinRegisterEvent`](#0x1_account_CoinRegisterEvent)
-  [Struct `CapabilityOffer`](#0x1_account_CapabilityOffer)
-  [Struct `RotationCapability`](#0x1_account_RotationCapability)
-  [Struct `SignerCapability`](#0x1_account_SignerCapability)
-  [Resource `OriginatingAddress`](#0x1_account_OriginatingAddress)
-  [Struct `RotationProofChallenge`](#0x1_account_RotationProofChallenge)
-  [Struct `RotationCapabilityOfferProofChallenge`](#0x1_account_RotationCapabilityOfferProofChallenge)
-  [Struct `SignerCapabilityOfferProofChallenge`](#0x1_account_SignerCapabilityOfferProofChallenge)
-  [Struct `RotationCapabilityOfferProofChallengeV2`](#0x1_account_RotationCapabilityOfferProofChallengeV2)
-  [Struct `SignerCapabilityOfferProofChallengeV2`](#0x1_account_SignerCapabilityOfferProofChallengeV2)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_account_initialize)
-  [Function `create_account_if_does_not_exist`](#0x1_account_create_account_if_does_not_exist)
-  [Function `create_account`](#0x1_account_create_account)
-  [Function `create_account_unchecked`](#0x1_account_create_account_unchecked)
-  [Function `exists_at`](#0x1_account_exists_at)
-  [Function `get_guid_next_creation_num`](#0x1_account_get_guid_next_creation_num)
-  [Function `get_sequence_number`](#0x1_account_get_sequence_number)
-  [Function `increment_sequence_number`](#0x1_account_increment_sequence_number)
-  [Function `get_authentication_key`](#0x1_account_get_authentication_key)
-  [Function `rotate_authentication_key_internal`](#0x1_account_rotate_authentication_key_internal)
-  [Function `rotate_authentication_key_call`](#0x1_account_rotate_authentication_key_call)
-  [Function `rotate_authentication_key`](#0x1_account_rotate_authentication_key)
-  [Function `rotate_authentication_key_with_rotation_capability`](#0x1_account_rotate_authentication_key_with_rotation_capability)
-  [Function `offer_rotation_capability`](#0x1_account_offer_rotation_capability)
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
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `create_account_if_does_not_exist`](#@Specification_1_create_account_if_does_not_exist)
    -  [Function `create_account`](#@Specification_1_create_account)
    -  [Function `create_account_unchecked`](#@Specification_1_create_account_unchecked)
    -  [Function `exists_at`](#@Specification_1_exists_at)
    -  [Function `get_guid_next_creation_num`](#@Specification_1_get_guid_next_creation_num)
    -  [Function `get_sequence_number`](#@Specification_1_get_sequence_number)
    -  [Function `increment_sequence_number`](#@Specification_1_increment_sequence_number)
    -  [Function `get_authentication_key`](#@Specification_1_get_authentication_key)
    -  [Function `rotate_authentication_key_internal`](#@Specification_1_rotate_authentication_key_internal)
    -  [Function `rotate_authentication_key_call`](#@Specification_1_rotate_authentication_key_call)
    -  [Function `rotate_authentication_key`](#@Specification_1_rotate_authentication_key)
    -  [Function `rotate_authentication_key_with_rotation_capability`](#@Specification_1_rotate_authentication_key_with_rotation_capability)
    -  [Function `offer_rotation_capability`](#@Specification_1_offer_rotation_capability)
    -  [Function `is_rotation_capability_offered`](#@Specification_1_is_rotation_capability_offered)
    -  [Function `get_rotation_capability_offer_for`](#@Specification_1_get_rotation_capability_offer_for)
    -  [Function `revoke_rotation_capability`](#@Specification_1_revoke_rotation_capability)
    -  [Function `revoke_any_rotation_capability`](#@Specification_1_revoke_any_rotation_capability)
    -  [Function `offer_signer_capability`](#@Specification_1_offer_signer_capability)
    -  [Function `is_signer_capability_offered`](#@Specification_1_is_signer_capability_offered)
    -  [Function `get_signer_capability_offer_for`](#@Specification_1_get_signer_capability_offer_for)
    -  [Function `revoke_signer_capability`](#@Specification_1_revoke_signer_capability)
    -  [Function `revoke_any_signer_capability`](#@Specification_1_revoke_any_signer_capability)
    -  [Function `create_authorized_signer`](#@Specification_1_create_authorized_signer)
    -  [Function `assert_valid_rotation_proof_signature_and_get_auth_key`](#@Specification_1_assert_valid_rotation_proof_signature_and_get_auth_key)
    -  [Function `update_auth_key_and_originating_address_table`](#@Specification_1_update_auth_key_and_originating_address_table)
    -  [Function `create_resource_address`](#@Specification_1_create_resource_address)
    -  [Function `create_resource_account`](#@Specification_1_create_resource_account)
    -  [Function `create_framework_reserved_account`](#@Specification_1_create_framework_reserved_account)
    -  [Function `create_guid`](#@Specification_1_create_guid)
    -  [Function `new_event_handle`](#@Specification_1_new_event_handle)
    -  [Function `register_coin`](#@Specification_1_register_coin)
    -  [Function `create_signer_with_capability`](#@Specification_1_create_signer_with_capability)
    -  [Function `verify_signed_message`](#@Specification_1_verify_signed_message)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;<br /><b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;<br /><b>use</b> <a href="create_signer.md#0x1_create_signer">0x1::create_signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs">0x1::from_bcs</a>;<br /><b>use</b> <a href="guid.md#0x1_guid">0x1::guid</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519">0x1::multi_ed25519</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_account_KeyRotation"></a>

## Struct `KeyRotation`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="account.md#0x1_account_KeyRotation">KeyRotation</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_account_Account"></a>

## Resource `Account`

Resource representing an account.


<pre><code><b>struct</b> <a href="account.md#0x1_account_Account">Account</a> <b>has</b> store, key<br /></code></pre>



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



<pre><code><b>struct</b> <a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a> <b>has</b> drop, store<br /></code></pre>



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

<a id="0x1_account_CapabilityOffer"></a>

## Struct `CapabilityOffer`



<pre><code><b>struct</b> <a href="account.md#0x1_account_CapabilityOffer">CapabilityOffer</a>&lt;T&gt; <b>has</b> store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationCapability">RotationCapability</a> <b>has</b> drop, store<br /></code></pre>



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



<pre><code><b>struct</b> <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> <b>has</b> drop, store<br /></code></pre>



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

For example, imagine a freshly&#45;created wallet with address <code>a</code> and thus also with authentication key <code>a</code>, derived from a PK <code>pk_a</code> with corresponding SK <code>sk_a</code>.
It is easy to recover such a wallet given just the secret key <code>sk_a</code>, since the PK can be derived from the SK, the authentication key can then be derived from the PK, and the address equals the authentication key (since there was no key rotation).

However, if such a wallet rotates its authentication key to <code>b</code> derived from a different PK <code>pk_b</code> with SK <code>sk_b</code>, how would account recovery work?
The recovered address would no longer be &apos;a&apos;; it would be <code>b</code>, which is incorrect.
This struct solves this problem by mapping the new authentication key <code>b</code> to the original address <code>a</code> and thus helping the wallet software during recovery find the correct address.


<pre><code><b>struct</b> <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> <b>has</b> key<br /></code></pre>



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
signed by the account owner&apos;s current public key, which proves possession of a capability to rotate the key.
Second, this struct is signed by the new public key that the account owner wants to rotate to, which proves
knowledge of this new public key&apos;s associated secret key. These two signatures cannot be replayed in another
context because they include the TXN&apos;s unique sequence number.


<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> <b>has</b> <b>copy</b>, drop<br /></code></pre>



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

Deprecated struct &#45; newest version is <code><a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a></code>


<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationCapabilityOfferProofChallenge">RotationCapabilityOfferProofChallenge</a> <b>has</b> drop<br /></code></pre>



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

Deprecated struct &#45; newest version is <code><a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a></code>


<pre><code><b>struct</b> <a href="account.md#0x1_account_SignerCapabilityOfferProofChallenge">SignerCapabilityOfferProofChallenge</a> <b>has</b> drop<br /></code></pre>



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


<pre><code><b>struct</b> <a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a> <b>has</b> drop<br /></code></pre>



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



<pre><code><b>struct</b> <a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a> <b>has</b> <b>copy</b>, drop<br /></code></pre>



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

<a id="@Constants_0"></a>

## Constants


<a id="0x1_account_MAX_U64"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>: u128 &#61; 18446744073709551615;<br /></code></pre>



<a id="0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME"></a>

Scheme identifier used when hashing an account&apos;s address together with a seed to derive the address (not the
authentication key) of a resource account. This is an abuse of the notion of a scheme identifier which, for now,
serves to domain separate hashes used to derive resource account addresses from hashes used to derive
authentication keys. Without such separation, an adversary could create (and get a signer for) a resource account
whose address matches an existing address of a MultiEd25519 wallet.


<pre><code><b>const</b> <a href="account.md#0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME">DERIVE_RESOURCE_ACCOUNT_SCHEME</a>: u8 &#61; 255;<br /></code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_EXISTS"></a>

Account already exists


<pre><code><b>const</b> <a href="account.md#0x1_account_EACCOUNT_ALREADY_EXISTS">EACCOUNT_ALREADY_EXISTS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_USED"></a>

An attempt to create a resource account on an account that has a committed transaction


<pre><code><b>const</b> <a href="account.md#0x1_account_EACCOUNT_ALREADY_USED">EACCOUNT_ALREADY_USED</a>: u64 &#61; 16;<br /></code></pre>



<a id="0x1_account_EACCOUNT_DOES_NOT_EXIST"></a>

Account does not exist


<pre><code><b>const</b> <a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_account_ECANNOT_RESERVED_ADDRESS"></a>

Cannot create account because address is reserved


<pre><code><b>const</b> <a href="account.md#0x1_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_account_ED25519_SCHEME"></a>

Scheme identifier for Ed25519 signatures used to derive authentication keys for Ed25519 public keys.


<pre><code><b>const</b> <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>: u8 &#61; 0;<br /></code></pre>



<a id="0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM">EEXCEEDED_MAX_GUID_CREATION_NUM</a>: u64 &#61; 20;<br /></code></pre>



<a id="0x1_account_EINVALID_ACCEPT_ROTATION_CAPABILITY"></a>

The caller does not have a valid rotation capability offer from the other account


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_ACCEPT_ROTATION_CAPABILITY">EINVALID_ACCEPT_ROTATION_CAPABILITY</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_account_EINVALID_ORIGINATING_ADDRESS"></a>

Abort the transaction if the expected originating address is different from the originating address on&#45;chain


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_ORIGINATING_ADDRESS">EINVALID_ORIGINATING_ADDRESS</a>: u64 &#61; 13;<br /></code></pre>



<a id="0x1_account_EINVALID_PROOF_OF_KNOWLEDGE"></a>

Specified proof of knowledge required to prove ownership of a public key is invalid


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_account_EINVALID_SCHEME"></a>

Specified scheme required to proceed with the smart contract operation &#45; can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1)


<pre><code><b>const</b> <a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>: u64 &#61; 12;<br /></code></pre>



<a id="0x1_account_EMALFORMED_AUTHENTICATION_KEY"></a>

The provided authentication key has an invalid length


<pre><code><b>const</b> <a href="account.md#0x1_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_account_ENO_CAPABILITY"></a>

The caller does not have a digital&#45;signature&#45;based capability to call this function


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_CAPABILITY">ENO_CAPABILITY</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_account_ENO_SIGNER_CAPABILITY_OFFERED"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_SIGNER_CAPABILITY_OFFERED">ENO_SIGNER_CAPABILITY_OFFERED</a>: u64 &#61; 19;<br /></code></pre>



<a id="0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER"></a>

The specified rotation capablity offer does not exist at the specified offerer address


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>: u64 &#61; 18;<br /></code></pre>



<a id="0x1_account_ENO_SUCH_SIGNER_CAPABILITY"></a>

The signer capability offer doesn&apos;t exist at the given address


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>: u64 &#61; 14;<br /></code></pre>



<a id="0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS"></a>

Address to create is not a valid reserved address for Aptos framework


<pre><code><b>const</b> <a href="account.md#0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS">ENO_VALID_FRAMEWORK_RESERVED_ADDRESS</a>: u64 &#61; 11;<br /></code></pre>



<a id="0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST"></a>

Offerer address doesn&apos;t exist


<pre><code><b>const</b> <a href="account.md#0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST">EOFFERER_ADDRESS_DOES_NOT_EXIST</a>: u64 &#61; 17;<br /></code></pre>



<a id="0x1_account_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code><b>const</b> <a href="account.md#0x1_account_EOUT_OF_GAS">EOUT_OF_GAS</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_account_ERESOURCE_ACCCOUNT_EXISTS"></a>

An attempt to create a resource account on a claimed account


<pre><code><b>const</b> <a href="account.md#0x1_account_ERESOURCE_ACCCOUNT_EXISTS">ERESOURCE_ACCCOUNT_EXISTS</a>: u64 &#61; 15;<br /></code></pre>



<a id="0x1_account_ESEQUENCE_NUMBER_TOO_BIG"></a>

Sequence number exceeds the maximum value for a u64


<pre><code><b>const</b> <a href="account.md#0x1_account_ESEQUENCE_NUMBER_TOO_BIG">ESEQUENCE_NUMBER_TOO_BIG</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_account_EWRONG_CURRENT_PUBLIC_KEY"></a>

Specified current public key is not correct


<pre><code><b>const</b> <a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_account_MAX_GUID_CREATION_NUM"></a>

Explicitly separate the GUID space between Object and Account to prevent accidental overlap.


<pre><code><b>const</b> <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">MAX_GUID_CREATION_NUM</a>: u64 &#61; 1125899906842624;<br /></code></pre>



<a id="0x1_account_MULTI_ED25519_SCHEME"></a>

Scheme identifier for MultiEd25519 signatures used to derive authentication keys for MultiEd25519 public keys.


<pre><code><b>const</b> <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>: u8 &#61; 1;<br /></code></pre>



<a id="0x1_account_ZERO_AUTH_KEY"></a>



<pre><code><b>const</b> <a href="account.md#0x1_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];<br /></code></pre>



<a id="0x1_account_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>move_to</b>(aptos_framework, <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> &#123;<br />        address_map: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_account_if_does_not_exist"></a>

## Function `create_account_if_does_not_exist`



<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>) &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address)) &#123;<br />        <a href="account.md#0x1_account_create_account">create_account</a>(account_address);<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_account"></a>

## Function `create_account`

Publishes a new <code><a href="account.md#0x1_account_Account">Account</a></code> resource under <code>new_address</code>. A signer representing <code>new_address</code>
is returned. This way, the caller of this function can publish additional resources under
<code>new_address</code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#123;<br />    // there cannot be an <a href="account.md#0x1_account_Account">Account</a> resource under new_addr already.<br />    <b>assert</b>!(!<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(new_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="account.md#0x1_account_EACCOUNT_ALREADY_EXISTS">EACCOUNT_ALREADY_EXISTS</a>));<br /><br />    // NOTE: @core_resources gets created via a `create_account` call, so we do not <b>include</b> it below.<br />    <b>assert</b>!(<br />        new_address !&#61; @vm_reserved &amp;&amp; new_address !&#61; @aptos_framework &amp;&amp; new_address !&#61; @aptos_token,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_ECANNOT_RESERVED_ADDRESS">ECANNOT_RESERVED_ADDRESS</a>)<br />    );<br /><br />    <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_account_unchecked"></a>

## Function `create_account_unchecked`



<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#123;<br />    <b>let</b> new_account &#61; <a href="create_signer.md#0x1_create_signer">create_signer</a>(new_address);<br />    <b>let</b> authentication_key &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&amp;new_address);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;authentication_key) &#61;&#61; 32,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>)<br />    );<br /><br />    <b>let</b> guid_creation_num &#61; 0;<br /><br />    <b>let</b> guid_for_coin &#61; <a href="guid.md#0x1_guid_create">guid::create</a>(new_address, &amp;<b>mut</b> guid_creation_num);<br />    <b>let</b> coin_register_events &#61; <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>&lt;<a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a>&gt;(guid_for_coin);<br /><br />    <b>let</b> guid_for_rotation &#61; <a href="guid.md#0x1_guid_create">guid::create</a>(new_address, &amp;<b>mut</b> guid_creation_num);<br />    <b>let</b> key_rotation_events &#61; <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>&lt;<a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a>&gt;(guid_for_rotation);<br /><br />    <b>move_to</b>(<br />        &amp;new_account,<br />        <a href="account.md#0x1_account_Account">Account</a> &#123;<br />            authentication_key,<br />            sequence_number: 0,<br />            guid_creation_num,<br />            coin_register_events,<br />            key_rotation_events,<br />            rotation_capability_offer: <a href="account.md#0x1_account_CapabilityOffer">CapabilityOffer</a> &#123; for: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() &#125;,<br />            signer_capability_offer: <a href="account.md#0x1_account_CapabilityOffer">CapabilityOffer</a> &#123; for: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() &#125;,<br />        &#125;<br />    );<br /><br />    new_account<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_exists_at"></a>

## Function `exists_at`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_exists_at">exists_at</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_exists_at">exists_at</a>(addr: <b>address</b>): bool &#123;<br />    <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_get_guid_next_creation_num"></a>

## Function `get_guid_next_creation_num`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_guid_next_creation_num">get_guid_next_creation_num</a>(addr: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_guid_next_creation_num">get_guid_next_creation_num</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).guid_creation_num<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_get_sequence_number"></a>

## Function `get_sequence_number`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_increment_sequence_number"></a>

## Function `increment_sequence_number`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> sequence_number &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number;<br /><br />    <b>assert</b>!(<br />        (&#42;sequence_number <b>as</b> u128) &lt; <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="account.md#0x1_account_ESEQUENCE_NUMBER_TOO_BIG">ESEQUENCE_NUMBER_TOO_BIG</a>)<br />    );<br /><br />    &#42;sequence_number &#61; &#42;sequence_number &#43; 1;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_get_authentication_key"></a>

## Function `get_authentication_key`



<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_authentication_key">get_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_authentication_key">get_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).authentication_key<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_internal"></a>

## Function `rotate_authentication_key_internal`

This function is used to rotate a resource account&apos;s authentication key to <code>new_auth_key</code>. This is done in
many contexts:
1. During normal key rotation via <code>rotate_authentication_key</code> or <code>rotate_authentication_key_call</code>
2. During resource account initialization so that no private key can control the resource account
3. During multisig_v2 account creation


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;new_auth_key) &#61;&#61; 32,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EMALFORMED_AUTHENTICATION_KEY">EMALFORMED_AUTHENTICATION_KEY</a>)<br />    );<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />    account_resource.authentication_key &#61; new_auth_key;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_call"></a>

## Function `rotate_authentication_key_call`

Private entry function for key rotation that allows the signer to update their authentication key.
Note that this does not update the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table because the <code>new_auth_key</code> is not &quot;verified&quot;: it
does not come with a proof&#45;of&#45;knowledge of the underlying SK. Nonetheless, we need this functionality due to
the introduction of non&#45;standard key algorithms, such as passkeys, which cannot produce proofs&#45;of&#45;knowledge in
the format expected in <code>rotate_authentication_key</code>.


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>, new_auth_key);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key"></a>

## Function `rotate_authentication_key`

Generic authentication key rotation function that allows the user to rotate their authentication key from any scheme to any scheme.
To authorize the rotation, we need two signatures:
&#45; the first signature <code>cap_rotate_key</code> refers to the signature by the account owner&apos;s current key on a valid <code><a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a></code>,
demonstrating that the user intends to and has the capability to rotate the authentication key of this account;
&#45; the second signature <code>cap_update_table</code> refers to the signature by the new key (that the account owner wants to rotate to) on a
valid <code><a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a></code>, demonstrating that the user owns the new private key, and has the authority to update the
<code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> map with the new address mapping <code>&lt;new_address, originating_address&gt;</code>.
To verify these two signatures, we need their corresponding public key and public key scheme: we use <code>from_scheme</code> and <code>from_public_key_bytes</code>
to verify <code>cap_rotate_key</code>, and <code>to_scheme</code> and <code>to_public_key_bytes</code> to verify <code>cap_update_table</code>.
A scheme of 0 refers to an Ed25519 key and a scheme of 1 refers to Multi&#45;Ed25519 keys.
<code>originating <b>address</b></code> refers to an account&apos;s original/first address.

Here is an example attack if we don&apos;t ask for the second signature <code>cap_update_table</code>:
Alice has rotated her account <code>addr_a</code> to <code>new_addr_a</code>. As a result, the following entry is created, to help Alice when recovering her wallet:
<code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[new_addr_a]</code> &#45;&gt; <code>addr_a</code>
Alice has had bad day: her laptop blew up and she needs to reset her account on a new one.
(Fortunately, she still has her secret key <code>new_sk_a</code> associated with her new address <code>new_addr_a</code>, so she can do this.)

But Bob likes to mess with Alice.
Bob creates an account <code>addr_b</code> and maliciously rotates it to Alice&apos;s new address <code>new_addr_a</code>. Since we are no longer checking a PoK,
Bob can easily do this.

Now, the table will be updated to make Alice&apos;s new address point to Bob&apos;s address: <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[new_addr_a]</code> &#45;&gt; <code>addr_b</code>.
When Alice recovers her account, her wallet will display the attacker&apos;s address (Bob&apos;s) <code>addr_b</code> as her address.
Now Alice will give <code>addr_b</code> to everyone to pay her, but the money will go to Bob.

Because we ask for a valid <code>cap_update_table</code>, this kind of attack is not possible. Bob would not have the secret key of Alice&apos;s address
to rotate his address to Alice&apos;s address in the first place.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key">rotate_authentication_key</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from_scheme: u8, from_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_rotate_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key">rotate_authentication_key</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    from_scheme: u8,<br />    from_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    to_scheme: u8,<br />    to_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    cap_rotate_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a>, <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><br />    // Verify the given `from_public_key_bytes` matches this <a href="account.md#0x1_account">account</a>&apos;s current authentication key.<br />    <b>if</b> (from_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) &#123;<br />        <b>let</b> from_pk &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(from_public_key_bytes);<br />        <b>let</b> from_auth_key &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;from_pk);<br />        <b>assert</b>!(<br />            account_resource.authentication_key &#61;&#61; from_auth_key,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)<br />        );<br />    &#125; <b>else</b> <b>if</b> (from_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) &#123;<br />        <b>let</b> from_pk &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(from_public_key_bytes);<br />        <b>let</b> from_auth_key &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;from_pk);<br />        <b>assert</b>!(<br />            account_resource.authentication_key &#61;&#61; from_auth_key,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unauthenticated">error::unauthenticated</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)<br />        );<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)<br />    &#125;;<br /><br />    // Construct a valid `<a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>` that `cap_rotate_key` and `cap_update_table` will validate against.<br />    <b>let</b> curr_auth_key_as_address &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(account_resource.authentication_key);<br />    <b>let</b> challenge &#61; <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> &#123;<br />        sequence_number: account_resource.sequence_number,<br />        originator: addr,<br />        current_auth_key: curr_auth_key_as_address,<br />        new_public_key: to_public_key_bytes,<br />    &#125;;<br /><br />    // Assert the challenges signed by the current and new keys are valid<br />    <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(<br />        from_scheme,<br />        from_public_key_bytes,<br />        cap_rotate_key,<br />        &amp;challenge<br />    );<br />    <b>let</b> new_auth_key &#61; <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(<br />        to_scheme,<br />        to_public_key_bytes,<br />        cap_update_table,<br />        &amp;challenge<br />    );<br /><br />    // Update the `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>` <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>.<br />    <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(addr, account_resource, new_auth_key);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_with_rotation_capability"></a>

## Function `rotate_authentication_key_with_rotation_capability`



<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_with_rotation_capability">rotate_authentication_key_with_rotation_capability</a>(delegate_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_cap_offerer_address: <b>address</b>, new_scheme: u8, new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_with_rotation_capability">rotate_authentication_key_with_rotation_capability</a>(<br />    delegate_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    rotation_cap_offerer_address: <b>address</b>,<br />    new_scheme: u8,<br />    new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br />) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a>, <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> &#123;<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(rotation_cap_offerer_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST">EOFFERER_ADDRESS_DOES_NOT_EXIST</a>));<br /><br />    // Check that there <b>exists</b> a rotation capability offer at the offerer&apos;s <a href="account.md#0x1_account">account</a> resource for the delegate.<br />    <b>let</b> delegate_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegate_signer);<br />    <b>let</b> offerer_account_resource &#61; <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_contains">option::contains</a>(&amp;offerer_account_resource.rotation_capability_offer.for, &amp;delegate_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>)<br />    );<br /><br />    <b>let</b> curr_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(offerer_account_resource.authentication_key);<br />    <b>let</b> challenge &#61; <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> &#123;<br />        sequence_number: <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(delegate_address),<br />        originator: rotation_cap_offerer_address,<br />        current_auth_key: curr_auth_key,<br />        new_public_key: new_public_key_bytes,<br />    &#125;;<br /><br />    // Verifies that the `<a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>` from above is signed under the new <b>public</b> key that we are rotating <b>to</b>.        l<br />    <b>let</b> new_auth_key &#61; <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(<br />        new_scheme,<br />        new_public_key_bytes,<br />        cap_update_table,<br />        &amp;challenge<br />    );<br /><br />    // Update the `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>` <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, so we can find the originating <b>address</b> using the new <b>address</b>.<br />    <b>let</b> offerer_account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address);<br />    <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(<br />        rotation_cap_offerer_address,<br />        offerer_account_resource,<br />        new_auth_key<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_offer_rotation_capability"></a>

## Function `offer_rotation_capability`

Offers rotation capability on behalf of <code><a href="account.md#0x1_account">account</a></code> to the account at address <code>recipient_address</code>.
An account can delegate its rotation capability to only one other address at one time. If the account
has an existing rotation capability offer, calling this function will update the rotation capability offer with
the new <code>recipient_address</code>.
Here, <code>rotation_capability_sig_bytes</code> signature indicates that this key rotation is authorized by the account owner,
and prevents the classic &quot;time&#45;of&#45;check time&#45;of&#45;use&quot; attack.
For example, users usually rely on what the wallet displays to them as the transaction&apos;s outcome. Consider a contract that with 50% probability
(based on the current timestamp in Move), rotates somebody&apos;s key. The wallet might be unlucky and get an outcome where nothing is rotated,
incorrectly telling the user nothing bad will happen. But when the transaction actually gets executed, the attacker gets lucky and
the execution path triggers the account key rotation.
We prevent such attacks by asking for this extra signature authorizing the key rotation.

@param rotation_capability_sig_bytes is the signature by the account owner&apos;s key on <code><a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a></code>.
@param account_scheme is the scheme of the account (ed25519 or multi_ed25519).
@param account_public_key_bytes is the public key of the account owner.
@param recipient_address is the address of the recipient of the rotation capability &#45; note that if there&apos;s an existing rotation capability
offer, calling this function will replace the previous <code>recipient_address</code> upon successful verification.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_rotation_capability">offer_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_rotation_capability">offer_rotation_capability</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    rotation_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    account_scheme: u8,<br />    account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    recipient_address: <b>address</b>,<br />) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(recipient_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));<br /><br />    // proof that this <a href="account.md#0x1_account">account</a> intends <b>to</b> delegate its rotation capability <b>to</b> another <a href="account.md#0x1_account">account</a><br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />    <b>let</b> proof_challenge &#61; <a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a> &#123;<br />        <a href="chain_id.md#0x1_chain_id">chain_id</a>: <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>(),<br />        sequence_number: account_resource.sequence_number,<br />        source_address: addr,<br />        recipient_address,<br />    &#125;;<br /><br />    // verify the signature on `<a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a>` by the <a href="account.md#0x1_account">account</a> owner<br />    <b>if</b> (account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) &#123;<br />        <b>let</b> pubkey &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key_bytes);<br />        <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;pubkey);<br />        <b>assert</b>!(<br />            account_resource.authentication_key &#61;&#61; expected_auth_key,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)<br />        );<br /><br />        <b>let</b> rotation_capability_sig &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_signature_from_bytes">ed25519::new_signature_from_bytes</a>(rotation_capability_sig_bytes);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict_t">ed25519::signature_verify_strict_t</a>(&amp;rotation_capability_sig, &amp;pubkey, proof_challenge),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)<br />        );<br />    &#125; <b>else</b> <b>if</b> (account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) &#123;<br />        <b>let</b> pubkey &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key_bytes);<br />        <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;pubkey);<br />        <b>assert</b>!(<br />            account_resource.authentication_key &#61;&#61; expected_auth_key,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>)<br />        );<br /><br />        <b>let</b> rotation_capability_sig &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">multi_ed25519::new_signature_from_bytes</a>(rotation_capability_sig_bytes);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">multi_ed25519::signature_verify_strict_t</a>(&amp;rotation_capability_sig, &amp;pubkey, proof_challenge),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)<br />        );<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)<br />    &#125;;<br /><br />    // <b>update</b> the existing rotation capability offer or put in a new rotation capability offer for the current <a href="account.md#0x1_account">account</a><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_swap_or_fill">option::swap_or_fill</a>(&amp;<b>mut</b> account_resource.rotation_capability_offer.for, recipient_address);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_is_rotation_capability_offered"></a>

## Function `is_rotation_capability_offered`

Returns true if the account at <code>account_addr</code> has a rotation capability offer.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_rotation_capability_offered">is_rotation_capability_offered</a>(account_addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_rotation_capability_offered">is_rotation_capability_offered</a>(account_addr: <b>address</b>): bool <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> account_resource &#61; <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;account_resource.rotation_capability_offer.for)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_get_rotation_capability_offer_for"></a>

## Function `get_rotation_capability_offer_for`

Returns the address of the account that has a rotation capability offer from the account at <code>account_addr</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_rotation_capability_offer_for">get_rotation_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_rotation_capability_offer_for">get_rotation_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> account_resource &#61; <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;account_resource.rotation_capability_offer.for),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SIGNER_CAPABILITY_OFFERED">ENO_SIGNER_CAPABILITY_OFFERED</a>),<br />    );<br />    &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;account_resource.rotation_capability_offer.for)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_revoke_rotation_capability"></a>

## Function `revoke_rotation_capability`

Revoke the rotation capability offer given to <code>to_be_revoked_recipient_address</code> from <code><a href="account.md#0x1_account">account</a></code>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_rotation_capability">revoke_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_rotation_capability">revoke_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(to_be_revoked_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_contains">option::contains</a>(&amp;account_resource.rotation_capability_offer.for, &amp;to_be_revoked_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER">ENO_SUCH_ROTATION_CAPABILITY_OFFER</a>)<br />    );<br />    <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_revoke_any_rotation_capability"></a>

## Function `revoke_any_rotation_capability`

Revoke any rotation capability offer in the specified account.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> account_resource.rotation_capability_offer.for);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_offer_signer_capability"></a>

## Function `offer_signer_capability`

Offers signer capability on behalf of <code><a href="account.md#0x1_account">account</a></code> to the account at address <code>recipient_address</code>.
An account can delegate its signer capability to only one other address at one time.
<code>signer_capability_key_bytes</code> is the <code><a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a></code> signed by the account owner&apos;s key
<code>account_scheme</code> is the scheme of the account (ed25519 or multi_ed25519).
<code>account_public_key_bytes</code> is the public key of the account owner.
<code>recipient_address</code> is the address of the recipient of the signer capability &#45; note that if there&apos;s an existing
<code>recipient_address</code> in the account owner&apos;s <code>SignerCapabilityOffer</code>, this will replace the
previous <code>recipient_address</code> upon successful verification (the previous recipient will no longer have access
to the account owner&apos;s signer capability).


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_signer_capability">offer_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_signer_capability">offer_signer_capability</a>(<br />    <a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,<br />    signer_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    account_scheme: u8,<br />    account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    recipient_address: <b>address</b><br />) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> source_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(recipient_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));<br /><br />    // Proof that this <a href="account.md#0x1_account">account</a> intends <b>to</b> delegate its <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> capability <b>to</b> another <a href="account.md#0x1_account">account</a>.<br />    <b>let</b> proof_challenge &#61; <a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a> &#123;<br />        sequence_number: <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(source_address),<br />        source_address,<br />        recipient_address,<br />    &#125;;<br />    <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>(<br />        source_address, account_scheme, account_public_key_bytes, signer_capability_sig_bytes, proof_challenge);<br /><br />    // Update the existing <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> capability offer or put in a new <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> capability offer for the recipient.<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_swap_or_fill">option::swap_or_fill</a>(&amp;<b>mut</b> account_resource.signer_capability_offer.for, recipient_address);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_is_signer_capability_offered"></a>

## Function `is_signer_capability_offered`

Returns true if the account at <code>account_addr</code> has a signer capability offer.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_signer_capability_offered">is_signer_capability_offered</a>(account_addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_signer_capability_offered">is_signer_capability_offered</a>(account_addr: <b>address</b>): bool <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> account_resource &#61; <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;account_resource.signer_capability_offer.for)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_get_signer_capability_offer_for"></a>

## Function `get_signer_capability_offer_for`

Returns the address of the account that has a signer capability offer from the account at <code>account_addr</code>.


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_offer_for">get_signer_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_offer_for">get_signer_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> account_resource &#61; <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;account_resource.signer_capability_offer.for),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SIGNER_CAPABILITY_OFFERED">ENO_SIGNER_CAPABILITY_OFFERED</a>),<br />    );<br />    &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;account_resource.signer_capability_offer.for)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_revoke_signer_capability"></a>

## Function `revoke_signer_capability`

Revoke the account owner&apos;s signer capability offer for <code>to_be_revoked_address</code> (i.e., the address that
has a signer capability offer from <code><a href="account.md#0x1_account">account</a></code> but will be revoked in this function).


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_signer_capability">revoke_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_signer_capability">revoke_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(to_be_revoked_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EACCOUNT_DOES_NOT_EXIST">EACCOUNT_DOES_NOT_EXIST</a>));<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_contains">option::contains</a>(&amp;account_resource.signer_capability_offer.for, &amp;to_be_revoked_address),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>)<br />    );<br />    <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_revoke_any_signer_capability"></a>

## Function `revoke_any_signer_capability`

Revoke any signer capability offer in the specified account.


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> account_resource.signer_capability_offer.for);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_authorized_signer"></a>

## Function `create_authorized_signer`

Return an authorized signer of the offerer, if there&apos;s an existing signer capability offer for <code><a href="account.md#0x1_account">account</a></code>
at the offerer&apos;s address.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_authorized_signer">create_authorized_signer</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offerer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_authorized_signer">create_authorized_signer</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offerer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>assert</b>!(<a href="account.md#0x1_account_exists_at">exists_at</a>(offerer_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST">EOFFERER_ADDRESS_DOES_NOT_EXIST</a>));<br /><br />    // Check <b>if</b> there&apos;s an existing <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> capability offer from the offerer.<br />    <b>let</b> account_resource &#61; <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(offerer_address);<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />    <b>assert</b>!(<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_contains">option::contains</a>(&amp;account_resource.signer_capability_offer.for, &amp;addr),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_ENO_SUCH_SIGNER_CAPABILITY">ENO_SUCH_SIGNER_CAPABILITY</a>)<br />    );<br /><br />    <a href="create_signer.md#0x1_create_signer">create_signer</a>(offerer_address)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

## Function `assert_valid_rotation_proof_signature_and_get_auth_key`

Helper functions for authentication key rotation.


<pre><code><b>fun</b> <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, challenge: &amp;<a href="account.md#0x1_account_RotationProofChallenge">account::RotationProofChallenge</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(<br />    scheme: u8,<br />    public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    challenge: &amp;<a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a><br />): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; &#123;<br />    <b>if</b> (scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) &#123;<br />        <b>let</b> pk &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(public_key_bytes);<br />        <b>let</b> sig &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_signature_from_bytes">ed25519::new_signature_from_bytes</a>(signature);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict_t">ed25519::signature_verify_strict_t</a>(&amp;sig, &amp;pk, &#42;challenge),<br />            std::error::invalid_argument(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)<br />        );<br />        <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;pk)<br />    &#125; <b>else</b> <b>if</b> (scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) &#123;<br />        <b>let</b> pk &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(public_key_bytes);<br />        <b>let</b> sig &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">multi_ed25519::new_signature_from_bytes</a>(signature);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">multi_ed25519::signature_verify_strict_t</a>(&amp;sig, &amp;pk, &#42;challenge),<br />            std::error::invalid_argument(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>)<br />        );<br />        <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;pk)<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_update_auth_key_and_originating_address_table"></a>

## Function `update_auth_key_and_originating_address_table`

Update the <code><a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a></code> table, so that we can find the originating address using the latest address
in the event of key recovery.


<pre><code><b>fun</b> <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(originating_addr: <b>address</b>, account_resource: &amp;<b>mut</b> <a href="account.md#0x1_account_Account">account::Account</a>, new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(<br />    originating_addr: <b>address</b>,<br />    account_resource: &amp;<b>mut</b> <a href="account.md#0x1_account_Account">Account</a>,<br />    new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />) <b>acquires</b> <a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a> &#123;<br />    <b>let</b> address_map &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map;<br />    <b>let</b> curr_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(account_resource.authentication_key);<br /><br />    // Checks `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[curr_auth_key]` is either unmapped, or mapped <b>to</b> `originating_address`.<br />    // If it&apos;s mapped <b>to</b> the originating <b>address</b>, removes that mapping.<br />    // Otherwise, <b>abort</b> <b>if</b> it&apos;s mapped <b>to</b> a different <b>address</b>.<br />    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(address_map, curr_auth_key)) &#123;<br />        // If account_a <b>with</b> address_a is rotating its keypair from keypair_a <b>to</b> keypair_b, we expect<br />        // the <b>address</b> of the <a href="account.md#0x1_account">account</a> <b>to</b> stay the same, <b>while</b> its keypair updates <b>to</b> keypair_b.<br />        // Here, by asserting that we&apos;re calling from the <a href="account.md#0x1_account">account</a> <b>with</b> the originating <b>address</b>, we enforce<br />        // the standard of keeping the same <b>address</b> and updating the keypair at the contract level.<br />        // Without this assertion, the dapps could also <b>update</b> the <a href="account.md#0x1_account">account</a>&apos;s <b>address</b> <b>to</b> address_b (the <b>address</b> that<br />        // is programmatically related <b>to</b> keypaier_b) and <b>update</b> the keypair <b>to</b> keypair_b. This causes problems<br />        // for interoperability because different dapps can implement this in different ways.<br />        // If the <a href="account.md#0x1_account">account</a> <b>with</b> <b>address</b> b calls this function <b>with</b> two valid signatures, it will <b>abort</b> at this step,<br />        // because <b>address</b> b is not the <a href="account.md#0x1_account">account</a>&apos;s originating <b>address</b>.<br />        <b>assert</b>!(<br />            originating_addr &#61;&#61; <a href="../../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(address_map, curr_auth_key),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="account.md#0x1_account_EINVALID_ORIGINATING_ADDRESS">EINVALID_ORIGINATING_ADDRESS</a>)<br />        );<br />    &#125;;<br /><br />    // Set `<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>[new_auth_key] &#61; originating_address`.<br />    <b>let</b> new_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(new_auth_key_vector);<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(address_map, new_auth_key, originating_addr);<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="account.md#0x1_account_KeyRotation">KeyRotation</a> &#123;<br />            <a href="account.md#0x1_account">account</a>: originating_addr,<br />            old_authentication_key: account_resource.authentication_key,<br />            new_authentication_key: new_auth_key_vector,<br />        &#125;);<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a>&gt;(<br />        &amp;<b>mut</b> account_resource.key_rotation_events,<br />        <a href="account.md#0x1_account_KeyRotationEvent">KeyRotationEvent</a> &#123;<br />            old_authentication_key: account_resource.authentication_key,<br />            new_authentication_key: new_auth_key_vector,<br />        &#125;<br />    );<br /><br />    // Update the <a href="account.md#0x1_account">account</a> resource&apos;s authentication key.<br />    account_resource.authentication_key &#61; new_auth_key_vector;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_resource_address"></a>

## Function `create_resource_address`

Basic account creation methods.
This is a helper function to compute resource addresses. Computation of the address
involves the use of a cryptographic hash operation and should be use thoughtfully.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(source: &amp;<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(source: &amp;<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b> &#123;<br />    <b>let</b> bytes &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(source);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&amp;<b>mut</b> bytes, seed);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> bytes, <a href="account.md#0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME">DERIVE_RESOURCE_ACCOUNT_SCHEME</a>);<br />    <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_to_address">from_bcs::to_address</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash_sha3_256">hash::sha3_256</a>(bytes))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_resource_account"></a>

## Function `create_resource_account`

A resource account is used to manage resources independent of an account managed by a user.
In Aptos a resource account is created based upon the sha3 256 of the source&apos;s address and additional seed data.
A resource account can only be created once, this is designated by setting the
<code>Account::signer_capability_offer::for</code> to the address of the resource account. While an entity may call
<code>create_account</code> to attempt to claim an account ahead of the creation of a resource account, if found Aptos will
transition ownership of the account over to the resource account. This is done by validating that the account has
yet to execute any transactions and that the <code>Account::signer_capability_offer::for</code> is none. The probability of a
collision where someone has legitimately produced a private key that maps to a resource account address is less
than <code>(1/2)^(256)</code>.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_account">create_resource_account</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_account">create_resource_account</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">SignerCapability</a>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> resource_addr &#61; <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(&amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source), seed);<br />    <b>let</b> resource &#61; <b>if</b> (<a href="account.md#0x1_account_exists_at">exists_at</a>(resource_addr)) &#123;<br />        <b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>borrow_global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(resource_addr);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&amp;<a href="account.md#0x1_account">account</a>.signer_capability_offer.for),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="account.md#0x1_account_ERESOURCE_ACCCOUNT_EXISTS">ERESOURCE_ACCCOUNT_EXISTS</a>),<br />        );<br />        <b>assert</b>!(<br />            <a href="account.md#0x1_account">account</a>.sequence_number &#61;&#61; 0,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="account.md#0x1_account_EACCOUNT_ALREADY_USED">EACCOUNT_ALREADY_USED</a>),<br />        );<br />        <a href="create_signer.md#0x1_create_signer">create_signer</a>(resource_addr)<br />    &#125; <b>else</b> &#123;<br />        <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(resource_addr)<br />    &#125;;<br /><br />    // By default, only the <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> should have control over the resource <a href="account.md#0x1_account">account</a> and not the auth key.<br />    // If the source <a href="account.md#0x1_account">account</a> wants direct control via auth key, they would need <b>to</b> explicitly rotate the auth key<br />    // of the resource <a href="account.md#0x1_account">account</a> using the <a href="account.md#0x1_account_SignerCapability">SignerCapability</a>.<br />    <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(&amp;resource, <a href="account.md#0x1_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>);<br /><br />    <b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(resource_addr);<br />    <a href="account.md#0x1_account">account</a>.signer_capability_offer.for &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(resource_addr);<br />    <b>let</b> signer_cap &#61; <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> &#123; <a href="account.md#0x1_account">account</a>: resource_addr &#125;;<br />    (resource, signer_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_framework_reserved_account"></a>

## Function `create_framework_reserved_account`

create the account for system reserved addresses


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_framework_reserved_account">create_framework_reserved_account</a>(addr: <b>address</b>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_framework_reserved_account">create_framework_reserved_account</a>(addr: <b>address</b>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">SignerCapability</a>) &#123;<br />    <b>assert</b>!(<br />        addr &#61;&#61; @0x1 &#124;&#124;<br />            addr &#61;&#61; @0x2 &#124;&#124;<br />            addr &#61;&#61; @0x3 &#124;&#124;<br />            addr &#61;&#61; @0x4 &#124;&#124;<br />            addr &#61;&#61; @0x5 &#124;&#124;<br />            addr &#61;&#61; @0x6 &#124;&#124;<br />            addr &#61;&#61; @0x7 &#124;&#124;<br />            addr &#61;&#61; @0x8 &#124;&#124;<br />            addr &#61;&#61; @0x9 &#124;&#124;<br />            addr &#61;&#61; @0xa,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="account.md#0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS">ENO_VALID_FRAMEWORK_RESERVED_ADDRESS</a>),<br />    );<br />    <b>let</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#61; <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(addr);<br />    <b>let</b> signer_cap &#61; <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> &#123; <a href="account.md#0x1_account">account</a>: addr &#125;;<br />    (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_cap)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_guid"></a>

## Function `create_guid`

GUID management methods.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_guid">create_guid</a>(account_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_guid">create_guid</a>(account_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a> <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(account_signer);<br />    <b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />    <b>let</b> <a href="guid.md#0x1_guid">guid</a> &#61; <a href="guid.md#0x1_guid_create">guid::create</a>(addr, &amp;<b>mut</b> <a href="account.md#0x1_account">account</a>.guid_creation_num);<br />    <b>assert</b>!(<br />        <a href="account.md#0x1_account">account</a>.guid_creation_num &lt; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">MAX_GUID_CREATION_NUM</a>,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="account.md#0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM">EEXCEEDED_MAX_GUID_CREATION_NUM</a>),<br />    );<br />    <a href="guid.md#0x1_guid">guid</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_new_event_handle"></a>

## Function `new_event_handle`

GUID management methods.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_new_event_handle">new_event_handle</a>&lt;T: drop &#43; store&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): EventHandle&lt;T&gt; <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <a href="event.md#0x1_event_new_event_handle">event::new_event_handle</a>(<a href="account.md#0x1_account_create_guid">create_guid</a>(<a href="account.md#0x1_account">account</a>))<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_register_coin"></a>

## Function `register_coin`

Coin management methods.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_register_coin">register_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_register_coin">register_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a>&gt;(<br />        &amp;<b>mut</b> <a href="account.md#0x1_account">account</a>.coin_register_events,<br />        <a href="account.md#0x1_account_CoinRegisterEvent">CoinRegisterEvent</a> &#123;<br />            <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">type_info</a>: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_of">type_info::type_of</a>&lt;CoinType&gt;(),<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_create_signer_with_capability"></a>

## Function `create_signer_with_capability`

Capability based functions for efficient use.


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_signer_with_capability">create_signer_with_capability</a>(capability: &amp;<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_signer_with_capability">create_signer_with_capability</a>(capability: &amp;<a href="account.md#0x1_account_SignerCapability">SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> &#123;<br />    <b>let</b> addr &#61; &amp;capability.<a href="account.md#0x1_account">account</a>;<br />    <a href="create_signer.md#0x1_create_signer">create_signer</a>(&#42;addr)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_get_signer_capability_address"></a>

## Function `get_signer_capability_address`



<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_address">get_signer_capability_address</a>(capability: &amp;<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>): <b>address</b><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_address">get_signer_capability_address</a>(capability: &amp;<a href="account.md#0x1_account_SignerCapability">SignerCapability</a>): <b>address</b> &#123;<br />    capability.<a href="account.md#0x1_account">account</a><br />&#125;<br /></code></pre>



</details>

<a id="0x1_account_verify_signed_message"></a>

## Function `verify_signed_message`



<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>&lt;T: drop&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, account_scheme: u8, account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signed_message_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: T)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>&lt;T: drop&gt;(<br />    <a href="account.md#0x1_account">account</a>: <b>address</b>,<br />    account_scheme: u8,<br />    account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    signed_message_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />    message: T,<br />) <b>acquires</b> <a href="account.md#0x1_account_Account">Account</a> &#123;<br />    <b>let</b> account_resource &#61; <b>borrow_global_mut</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>);<br />    // Verify that the `<a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a>` <b>has</b> the right information and is signed by the <a href="account.md#0x1_account">account</a> owner&apos;s key<br />    <b>if</b> (account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a>) &#123;<br />        <b>let</b> pubkey &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_unvalidated_public_key_from_bytes">ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key);<br />        <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_unvalidated_public_key_to_authentication_key">ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;pubkey);<br />        <b>assert</b>!(<br />            account_resource.authentication_key &#61;&#61; expected_auth_key,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>),<br />        );<br /><br />        <b>let</b> signer_capability_sig &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_signature_from_bytes">ed25519::new_signature_from_bytes</a>(signed_message_bytes);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_signature_verify_strict_t">ed25519::signature_verify_strict_t</a>(&amp;signer_capability_sig, &amp;pubkey, message),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>),<br />        );<br />    &#125; <b>else</b> <b>if</b> (account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>) &#123;<br />        <b>let</b> pubkey &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_unvalidated_public_key_from_bytes">multi_ed25519::new_unvalidated_public_key_from_bytes</a>(account_public_key);<br />        <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_unvalidated_public_key_to_authentication_key">multi_ed25519::unvalidated_public_key_to_authentication_key</a>(&amp;pubkey);<br />        <b>assert</b>!(<br />            account_resource.authentication_key &#61;&#61; expected_auth_key,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EWRONG_CURRENT_PUBLIC_KEY">EWRONG_CURRENT_PUBLIC_KEY</a>),<br />        );<br /><br />        <b>let</b> signer_capability_sig &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_new_signature_from_bytes">multi_ed25519::new_signature_from_bytes</a>(signed_message_bytes);<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_signature_verify_strict_t">multi_ed25519::signature_verify_strict_t</a>(&amp;signer_capability_sig, &amp;pubkey, message),<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_PROOF_OF_KNOWLEDGE">EINVALID_PROOF_OF_KNOWLEDGE</a>),<br />        );<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="account.md#0x1_account_EINVALID_SCHEME">EINVALID_SCHEME</a>)<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

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
<td>In the rotate_authentication_key function, the authentication key derived from the from_public_key_bytes should match the signer&apos;s current authentication key. Only the delegate_signer granted the rotation capabilities may invoke the rotate_authentication_key_with_rotation_capability function.</td>
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
<td>The rotation of the authentication key updates the account&apos;s authentication key with the newly supplied one.</td>
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


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


Only the address <code>@aptos_framework</code> can call.
OriginatingAddress does not exist under <code>@aptos_framework</code> before the call.


<pre><code><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_addr);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(aptos_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(aptos_addr);<br /></code></pre>



<a id="@Specification_1_create_account_if_does_not_exist"></a>

### Function `create_account_if_does_not_exist`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)<br /></code></pre>


Ensure that the account exists at the end of the call.


<pre><code><b>let</b> authentication_key &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(account_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address) &amp;&amp; (<br />    account_address &#61;&#61; @vm_reserved<br />    &#124;&#124; account_address &#61;&#61; @aptos_framework<br />    &#124;&#124; account_address &#61;&#61; @aptos_token<br />    &#124;&#124; !(len(authentication_key) &#61;&#61; 32)<br />);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address);<br /></code></pre>



<a id="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_account">create_account</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.
Limit the new account address is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code><b>include</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> &#123;addr: new_address&#125;;<br /><b>aborts_if</b> new_address &#61;&#61; @vm_reserved &#124;&#124; new_address &#61;&#61; @aptos_framework &#124;&#124; new_address &#61;&#61; @aptos_token;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) &#61;&#61; new_address;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(new_address);<br /></code></pre>



<a id="@Specification_1_create_account_unchecked"></a>

### Function `create_account_unchecked`


<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_unchecked">create_account_unchecked</a>(new_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.


<pre><code><b>include</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> &#123;addr: new_address&#125;;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) &#61;&#61; new_address;<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(new_address);<br /></code></pre>



<a id="@Specification_1_exists_at"></a>

### Function `exists_at`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_exists_at">exists_at</a>(addr: <b>address</b>): bool<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>aborts_if</b> <b>false</b>;<br /></code></pre>




<a id="0x1_account_CreateAccountAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> &#123;<br />addr: <b>address</b>;<br /><b>let</b> authentication_key &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(addr);<br /><b>aborts_if</b> len(authentication_key) !&#61; 32;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> len(authentication_key) &#61;&#61; 32;<br />&#125;<br /></code></pre>



<a id="@Specification_1_get_guid_next_creation_num"></a>

### Function `get_guid_next_creation_num`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_guid_next_creation_num">get_guid_next_creation_num</a>(addr: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> result &#61;&#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).guid_creation_num;<br /></code></pre>



<a id="@Specification_1_get_sequence_number"></a>

### Function `get_sequence_number`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_sequence_number">get_sequence_number</a>(addr: <b>address</b>): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> result &#61;&#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number;<br /></code></pre>



<a id="@Specification_1_increment_sequence_number"></a>

### Function `increment_sequence_number`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_increment_sequence_number">increment_sequence_number</a>(addr: <b>address</b>)<br /></code></pre>


The Account existed under the address.
The sequence_number of the Account is up to MAX_U64.


<pre><code><b>let</b> sequence_number &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>aborts_if</b> sequence_number &#61;&#61; <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>;<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>let</b> <b>post</b> post_sequence_number &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).sequence_number;<br /><b>ensures</b> post_sequence_number &#61;&#61; sequence_number &#43; 1;<br /></code></pre>



<a id="@Specification_1_get_authentication_key"></a>

### Function `get_authentication_key`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_authentication_key">get_authentication_key</a>(addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> result &#61;&#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).authentication_key;<br /></code></pre>



<a id="@Specification_1_rotate_authentication_key_internal"></a>

### Function `rotate_authentication_key_internal`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_internal">rotate_authentication_key_internal</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>


The Account existed under the signer before the call.
The length of new_auth_key is 32.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />// This enforces <a id="high-level-req-10" href="#high-level-req">high&#45;level requirement 10</a>:
<b>let</b> <b>post</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(new_auth_key) !&#61; 32;<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> account_resource.authentication_key &#61;&#61; new_auth_key;<br /></code></pre>



<a id="@Specification_1_rotate_authentication_key_call"></a>

### Function `rotate_authentication_key_call`


<pre><code>entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_call">rotate_authentication_key_call</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, new_auth_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br />// This enforces <a id="high-level-req-10" href="#high-level-req">high&#45;level requirement 10</a>:
<b>let</b> <b>post</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(new_auth_key) !&#61; 32;<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> account_resource.authentication_key &#61;&#61; new_auth_key;<br /></code></pre>




<a id="0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, challenge: <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /></code></pre>



<a id="@Specification_1_rotate_authentication_key"></a>

### Function `rotate_authentication_key`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key">rotate_authentication_key</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, from_scheme: u8, from_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_rotate_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>


The Account existed under the signer
The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME


<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />// This enforces <a id="high-level-req-6.1" href="#high-level-req">high&#45;level requirement 6</a>:
<b>include</b> from_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: from_public_key_bytes &#125;;<br /><b>aborts_if</b> from_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(from_public_key_bytes);<br />    account_resource.authentication_key !&#61; expected_auth_key<br />&#125;);<br /><b>include</b> from_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: from_public_key_bytes &#125;;<br /><b>aborts_if</b> from_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> from_auth_key &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(from_public_key_bytes);<br />    account_resource.authentication_key !&#61; from_auth_key<br />&#125;);<br />// This enforces <a id="high-level-req-5.1" href="#high-level-req">high&#45;level requirement 5</a>:
<b>aborts_if</b> from_scheme !&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; from_scheme !&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;<br /><b>let</b> curr_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);<br /><b>let</b> challenge &#61; <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> &#123;<br />    sequence_number: account_resource.sequence_number,<br />    originator: addr,<br />    current_auth_key: curr_auth_key,<br />    new_public_key: to_public_key_bytes,<br />&#125;;<br />// This enforces <a id="high-level-req-9.1" href="#high-level-req">high&#45;level requirement 9</a>:
<b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> &#123;<br />    scheme: from_scheme,<br />    public_key_bytes: from_public_key_bytes,<br />    signature: cap_rotate_key,<br />    challenge,<br />&#125;;<br /><b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> &#123;<br />    scheme: to_scheme,<br />    public_key_bytes: to_public_key_bytes,<br />    signature: cap_update_table,<br />    challenge,<br />&#125;;<br /><b>let</b> originating_addr &#61; addr;<br /><b>let</b> new_auth_key_vector &#61; <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(to_scheme, to_public_key_bytes, cap_update_table, challenge);<br /><b>let</b> address_map &#61; <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map;<br /><b>let</b> new_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, curr_auth_key) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(address_map, curr_auth_key) !&#61; originating_addr;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(new_auth_key_vector);<br /><b>aborts_if</b> curr_auth_key !&#61; new_auth_key &amp;&amp; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, new_auth_key);<br /><b>include</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a> &#123;<br />    originating_addr: addr,<br />&#125;;<br /><b>let</b> <b>post</b> auth_key &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).authentication_key;<br /><b>ensures</b> auth_key &#61;&#61; new_auth_key_vector;<br /></code></pre>



<a id="@Specification_1_rotate_authentication_key_with_rotation_capability"></a>

### Function `rotate_authentication_key_with_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_rotate_authentication_key_with_rotation_capability">rotate_authentication_key_with_rotation_capability</a>(delegate_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_cap_offerer_address: <b>address</b>, new_scheme: u8, new_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cap_update_table: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address);<br /><b>let</b> delegate_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(delegate_signer);<br /><b>let</b> offerer_account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(offerer_account_resource.authentication_key);<br /><b>let</b> curr_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(offerer_account_resource.authentication_key);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(delegate_address);<br /><b>let</b> challenge &#61; <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a> &#123;<br />    sequence_number: <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(delegate_address).sequence_number,<br />    originator: rotation_cap_offerer_address,<br />    current_auth_key: curr_auth_key,<br />    new_public_key: new_public_key_bytes,<br />&#125;;<br />// This enforces <a id="high-level-req-6.2" href="#high-level-req">high&#45;level requirement 6</a>:
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(offerer_account_resource.rotation_capability_offer.for, delegate_address);<br />// This enforces <a id="high-level-req-9.1" href="#high-level-req">high&#45;level requirement 9</a>:
<b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> &#123;<br />    scheme: new_scheme,<br />    public_key_bytes: new_public_key_bytes,<br />    signature: cap_update_table,<br />    challenge,<br />&#125;;<br /><b>let</b> new_auth_key_vector &#61; <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(new_scheme, new_public_key_bytes, cap_update_table, challenge);<br /><b>let</b> address_map &#61; <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(offerer_account_resource.authentication_key);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, curr_auth_key) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(address_map, curr_auth_key) !&#61; rotation_cap_offerer_address;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(new_auth_key_vector);<br /><b>let</b> new_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector);<br /><b>aborts_if</b> curr_auth_key !&#61; new_auth_key &amp;&amp; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, new_auth_key);<br /><b>include</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a> &#123;<br />    originating_addr: rotation_cap_offerer_address,<br />    account_resource: offerer_account_resource,<br />&#125;;<br /><b>let</b> <b>post</b> auth_key &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(rotation_cap_offerer_address).authentication_key;<br /><b>ensures</b> auth_key &#61;&#61; new_auth_key_vector;<br /></code></pre>



<a id="@Specification_1_offer_rotation_capability"></a>

### Function `offer_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_rotation_capability">offer_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, rotation_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)<br /></code></pre>




<pre><code><b>let</b> source_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);<br /><b>let</b> proof_challenge &#61; <a href="account.md#0x1_account_RotationCapabilityOfferProofChallengeV2">RotationCapabilityOfferProofChallengeV2</a> &#123;<br />    <a href="chain_id.md#0x1_chain_id">chain_id</a>: <b>global</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">chain_id::ChainId</a>&gt;(@aptos_framework).id,<br />    sequence_number: account_resource.sequence_number,<br />    source_address,<br />    recipient_address,<br />&#125;;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="chain_id.md#0x1_chain_id_ChainId">chain_id::ChainId</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(recipient_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: account_public_key_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);<br />    account_resource.authentication_key !&#61; expected_auth_key<br />&#125;);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: rotation_capability_sig_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; !<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_signature_verify_strict_t">ed25519::spec_signature_verify_strict_t</a>(<br />    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_Signature">ed25519::Signature</a> &#123; bytes: rotation_capability_sig_bytes &#125;,<br />    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a> &#123; bytes: account_public_key_bytes &#125;,<br />    proof_challenge<br />);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: account_public_key_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);<br />    account_resource.authentication_key !&#61; expected_auth_key<br />&#125;);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: rotation_capability_sig_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &amp;&amp; !<a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_t">multi_ed25519::spec_signature_verify_strict_t</a>(<br />    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a> &#123; bytes: rotation_capability_sig_bytes &#125;,<br />    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a> &#123; bytes: account_public_key_bytes &#125;,<br />    proof_challenge<br />);<br />// This enforces <a id="high-level-req-5.2" href="#high-level-req">high&#45;level requirement 5</a>:
<b>aborts_if</b> account_scheme !&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; account_scheme !&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;<br />// This enforces <a id="high-level-req-7.1" href="#high-level-req">high&#45;level requirement 7</a>:
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);<br /><b>let</b> <b>post</b> offer_for &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address).rotation_capability_offer.for;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(offer_for) &#61;&#61; recipient_address;<br /></code></pre>



<a id="@Specification_1_is_rotation_capability_offered"></a>

### Function `is_rotation_capability_offered`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_rotation_capability_offered">is_rotation_capability_offered</a>(account_addr: <b>address</b>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /></code></pre>



<a id="@Specification_1_get_rotation_capability_offer_for"></a>

### Function `get_rotation_capability_offer_for`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_rotation_capability_offer_for">get_rotation_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /><b>aborts_if</b> len(account_resource.rotation_capability_offer.for.vec) &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_revoke_rotation_capability"></a>

### Function `revoke_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_rotation_capability">revoke_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(account_resource.rotation_capability_offer.for,to_be_revoked_address);<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);<br /><b>let</b> <b>post</b> offer_for &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).rotation_capability_offer.for;<br /><b>ensures</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(offer_for);<br /></code></pre>



<a id="@Specification_1_revoke_any_rotation_capability"></a>

### Function `revoke_any_rotation_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_rotation_capability">revoke_any_rotation_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />// This enforces <a id="high-level-req-7.3" href="#high-level-req">high&#45;level requirement 7</a>:
<b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(account_resource.rotation_capability_offer.for);<br /><b>let</b> <b>post</b> offer_for &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).rotation_capability_offer.for;<br /><b>ensures</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(offer_for);<br /></code></pre>



<a id="@Specification_1_offer_signer_capability"></a>

### Function `offer_signer_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_offer_signer_capability">offer_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, signer_capability_sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, recipient_address: <b>address</b>)<br /></code></pre>


The Account existed under the signer.
The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME.


<pre><code><b>let</b> source_address &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);<br /><b>let</b> proof_challenge &#61; <a href="account.md#0x1_account_SignerCapabilityOfferProofChallengeV2">SignerCapabilityOfferProofChallengeV2</a> &#123;<br />    sequence_number: account_resource.sequence_number,<br />    source_address,<br />    recipient_address,<br />&#125;;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(recipient_address);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: account_public_key_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);<br />    account_resource.authentication_key !&#61; expected_auth_key<br />&#125;);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: signer_capability_sig_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; !<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_signature_verify_strict_t">ed25519::spec_signature_verify_strict_t</a>(<br />    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_Signature">ed25519::Signature</a> &#123; bytes: signer_capability_sig_bytes &#125;,<br />    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a> &#123; bytes: account_public_key_bytes &#125;,<br />    proof_challenge<br />);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: account_public_key_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key_bytes);<br />    account_resource.authentication_key !&#61; expected_auth_key<br />&#125;);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: signer_capability_sig_bytes &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &amp;&amp; !<a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_t">multi_ed25519::spec_signature_verify_strict_t</a>(<br />    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a> &#123; bytes: signer_capability_sig_bytes &#125;,<br />    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a> &#123; bytes: account_public_key_bytes &#125;,<br />    proof_challenge<br />);<br />// This enforces <a id="high-level-req-5.3" href="#high-level-req">high&#45;level requirement 5</a>:
<b>aborts_if</b> account_scheme !&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; account_scheme !&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;<br />// This enforces <a id="high-level-req-7.2" href="#high-level-req">high&#45;level requirement 7</a>:
<b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address);<br /><b>let</b> <b>post</b> offer_for &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(source_address).signer_capability_offer.for;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(offer_for) &#61;&#61; recipient_address;<br /></code></pre>



<a id="@Specification_1_is_signer_capability_offered"></a>

### Function `is_signer_capability_offered`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_is_signer_capability_offered">is_signer_capability_offered</a>(account_addr: <b>address</b>): bool<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /></code></pre>



<a id="@Specification_1_get_signer_capability_offer_for"></a>

### Function `get_signer_capability_offer_for`


<pre><code>&#35;[view]<br /><b>public</b> <b>fun</b> <a href="account.md#0x1_account_get_signer_capability_offer_for">get_signer_capability_offer_for</a>(account_addr: <b>address</b>): <b>address</b><br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /><b>aborts_if</b> len(account_resource.signer_capability_offer.for.vec) &#61;&#61; 0;<br /></code></pre>



<a id="@Specification_1_revoke_signer_capability"></a>

### Function `revoke_signer_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_signer_capability">revoke_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, to_be_revoked_address: <b>address</b>)<br /></code></pre>


The Account existed under the signer.
The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address.


<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(account_resource.signer_capability_offer.for,to_be_revoked_address);<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(to_be_revoked_address);<br /></code></pre>



<a id="@Specification_1_revoke_any_signer_capability"></a>

### Function `revoke_any_signer_capability`


<pre><code><b>public</b> entry <b>fun</b> <a href="account.md#0x1_account_revoke_any_signer_capability">revoke_any_signer_capability</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br />// This enforces <a id="high-level-req-7.4" href="#high-level-req">high&#45;level requirement 7</a>:
<b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>));<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(account_resource.signer_capability_offer.for);<br /></code></pre>



<a id="@Specification_1_create_authorized_signer"></a>

### Function `create_authorized_signer`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_authorized_signer">create_authorized_signer</a>(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, offerer_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>


The Account existed under the signer.
The value of signer_capability_offer.for of Account resource under the signer is offerer_address.


<pre><code>// This enforces <a id="high-level-req-8" href="#high-level-req">high&#45;level requirement 8</a>:
<b>include</b> <a href="account.md#0x1_account_AccountContainsAddr">AccountContainsAddr</a>&#123;<br />    <a href="account.md#0x1_account">account</a>,<br />    <b>address</b>: offerer_address,<br />&#125;;<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(offerer_address);<br /><b>ensures</b> <b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(offerer_address);<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) &#61;&#61; offerer_address;<br /></code></pre>




<a id="0x1_account_AccountContainsAddr"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_AccountContainsAddr">AccountContainsAddr</a> &#123;<br /><a href="account.md#0x1_account">account</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>address</b>: <b>address</b>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<b>address</b>);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<b>address</b>);<br />// This enforces <a id="high-level-spec-3" href="create_signer.md#high-level-req">high&#45;level requirement 3</a> of the <a href="create_signer.md">create_signer</a> module:
    <b>aborts_if</b> !<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_contains">option::spec_contains</a>(account_resource.signer_capability_offer.for,addr);<br />&#125;<br /></code></pre>



<a id="@Specification_1_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

### Function `assert_valid_rotation_proof_signature_and_get_auth_key`


<pre><code><b>fun</b> <a href="account.md#0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key">assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme: u8, public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, challenge: &amp;<a href="account.md#0x1_account_RotationProofChallenge">account::RotationProofChallenge</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>include</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="account.md#0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key">spec_assert_valid_rotation_proof_signature_and_get_auth_key</a>(scheme, public_key_bytes, signature, challenge);<br /></code></pre>




<a id="0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf">AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf</a> &#123;<br />scheme: u8;<br />public_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />signature: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br />challenge: <a href="account.md#0x1_account_RotationProofChallenge">RotationProofChallenge</a>;<br /><b>include</b> scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: public_key_bytes &#125;;<br /><b>include</b> scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: signature &#125;;<br /><b>aborts_if</b> scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; !<a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_signature_verify_strict_t">ed25519::spec_signature_verify_strict_t</a>(<br />    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_Signature">ed25519::Signature</a> &#123; bytes: signature &#125;,<br />    <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_UnvalidatedPublicKey">ed25519::UnvalidatedPublicKey</a> &#123; bytes: public_key_bytes &#125;,<br />    challenge<br />);<br /><b>include</b> scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: public_key_bytes &#125;;<br /><b>include</b> scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: signature &#125;;<br /><b>aborts_if</b> scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &amp;&amp; !<a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_signature_verify_strict_t">multi_ed25519::spec_signature_verify_strict_t</a>(<br />    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_Signature">multi_ed25519::Signature</a> &#123; bytes: signature &#125;,<br />    <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_UnvalidatedPublicKey">multi_ed25519::UnvalidatedPublicKey</a> &#123; bytes: public_key_bytes &#125;,<br />    challenge<br />);<br /><b>aborts_if</b> scheme !&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; scheme !&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_update_auth_key_and_originating_address_table"></a>

### Function `update_auth_key_and_originating_address_table`


<pre><code><b>fun</b> <a href="account.md#0x1_account_update_auth_key_and_originating_address_table">update_auth_key_and_originating_address_table</a>(originating_addr: <b>address</b>, account_resource: &amp;<b>mut</b> <a href="account.md#0x1_account_Account">account::Account</a>, new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);<br /><b>include</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a>;<br /></code></pre>




<a id="0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf">UpdateAuthKeyAndOriginatingAddressTableAbortsIf</a> &#123;<br />originating_addr: <b>address</b>;<br />account_resource: <a href="account.md#0x1_account_Account">Account</a>;<br />new_auth_key_vector: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;<br /><b>let</b> address_map &#61; <b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map;<br /><b>let</b> curr_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);<br /><b>let</b> new_auth_key &#61; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(account_resource.authentication_key);<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, curr_auth_key) &amp;&amp;<br />    <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_get">table::spec_get</a>(address_map, curr_auth_key) !&#61; originating_addr;<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<b>address</b>&gt;(new_auth_key_vector);<br /><b>aborts_if</b> curr_auth_key !&#61; new_auth_key &amp;&amp; <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(address_map, new_auth_key);<br /><b>ensures</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table_spec_contains">table::spec_contains</a>(<b>global</b>&lt;<a href="account.md#0x1_account_OriginatingAddress">OriginatingAddress</a>&gt;(@aptos_framework).address_map, <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<b>address</b>&gt;(new_auth_key_vector));<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_resource_address"></a>

### Function `create_resource_address`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_address">create_resource_address</a>(source: &amp;<b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b><br /></code></pre>


The Account existed under the signer
The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address


<pre><code><b>pragma</b> opaque;<br /><b>pragma</b> aborts_if_is_strict &#61; <b>false</b>;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] result &#61;&#61; <a href="account.md#0x1_account_spec_create_resource_address">spec_create_resource_address</a>(source, seed);<br /></code></pre>




<a id="0x1_account_spec_create_resource_address"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_create_resource_address">spec_create_resource_address</a>(source: <b>address</b>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <b>address</b>;<br /></code></pre>



<a id="@Specification_1_create_resource_account"></a>

### Function `create_resource_account`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_resource_account">create_resource_account</a>(source: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>




<pre><code><b>let</b> source_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(source);<br /><b>let</b> resource_addr &#61; <a href="account.md#0x1_account_spec_create_resource_address">spec_create_resource_address</a>(source_addr, seed);<br /><b>aborts_if</b> len(<a href="account.md#0x1_account_ZERO_AUTH_KEY">ZERO_AUTH_KEY</a>) !&#61; 32;<br /><b>include</b> <a href="account.md#0x1_account_exists_at">exists_at</a>(resource_addr) &#61;&#61;&gt; <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">CreateResourceAccountAbortsIf</a>;<br /><b>include</b> !<a href="account.md#0x1_account_exists_at">exists_at</a>(resource_addr) &#61;&#61;&gt; <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> &#123;addr: resource_addr&#125;;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result_1) &#61;&#61; resource_addr;<br /><b>let</b> <b>post</b> offer_for &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(resource_addr).signer_capability_offer.for;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_borrow">option::spec_borrow</a>(offer_for) &#61;&#61; resource_addr;<br /><b>ensures</b> result_2 &#61;&#61; <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> &#123; <a href="account.md#0x1_account">account</a>: resource_addr &#125;;<br /></code></pre>



<a id="@Specification_1_create_framework_reserved_account"></a>

### Function `create_framework_reserved_account`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_create_framework_reserved_account">create_framework_reserved_account</a>(addr: <b>address</b>): (<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, <a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>)<br /></code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.
The system reserved addresses is @0x1 / @0x2 / @0x3 / @0x4 / @0x5  / @0x6 / @0x7 / @0x8 / @0x9 / @0xa.


<pre><code><b>aborts_if</b> <a href="account.md#0x1_account_spec_is_framework_address">spec_is_framework_address</a>(addr);<br /><b>include</b> <a href="account.md#0x1_account_CreateAccountAbortsIf">CreateAccountAbortsIf</a> &#123;addr&#125;;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result_1) &#61;&#61; addr;<br /><b>ensures</b> result_2 &#61;&#61; <a href="account.md#0x1_account_SignerCapability">SignerCapability</a> &#123; <a href="account.md#0x1_account">account</a>: addr &#125;;<br /></code></pre>




<a id="0x1_account_spec_is_framework_address"></a>


<pre><code><b>fun</b> <a href="account.md#0x1_account_spec_is_framework_address">spec_is_framework_address</a>(addr: <b>address</b>): bool&#123;<br />   addr !&#61; @0x1 &amp;&amp;<br />   addr !&#61; @0x2 &amp;&amp;<br />   addr !&#61; @0x3 &amp;&amp;<br />   addr !&#61; @0x4 &amp;&amp;<br />   addr !&#61; @0x5 &amp;&amp;<br />   addr !&#61; @0x6 &amp;&amp;<br />   addr !&#61; @0x7 &amp;&amp;<br />   addr !&#61; @0x8 &amp;&amp;<br />   addr !&#61; @0x9 &amp;&amp;<br />   addr !&#61; @0xa<br />&#125;<br /></code></pre>



<a id="@Specification_1_create_guid"></a>

### Function `create_guid`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_guid">create_guid</a>(account_signer: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="guid.md#0x1_guid_GUID">guid::GUID</a><br /></code></pre>


The Account existed under the signer.
The guid_creation_num of the ccount resource is up to MAX_U64.


<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(account_signer);<br /><b>include</b> <a href="account.md#0x1_account_NewEventHandleAbortsIf">NewEventHandleAbortsIf</a> &#123;<br />    <a href="account.md#0x1_account">account</a>: account_signer,<br />&#125;;<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br />// This enforces <a id="high-level-req-11" href="#high-level-req">high&#45;level requirement 11</a>:
<b>ensures</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).guid_creation_num &#61;&#61; <b>old</b>(<b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr).guid_creation_num) &#43; 1;<br /></code></pre>



<a id="@Specification_1_new_event_handle"></a>

### Function `new_event_handle`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_new_event_handle">new_event_handle</a>&lt;T: drop, store&gt;(<a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;T&gt;<br /></code></pre>


The Account existed under the signer.
The guid_creation_num of the Account is up to MAX_U64.


<pre><code><b>include</b> <a href="account.md#0x1_account_NewEventHandleAbortsIf">NewEventHandleAbortsIf</a>;<br /></code></pre>




<a id="0x1_account_NewEventHandleAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_NewEventHandleAbortsIf">NewEventHandleAbortsIf</a> &#123;<br /><a href="account.md#0x1_account">account</a>: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(addr);<br /><b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num &#43; 1 &gt; <a href="account.md#0x1_account_MAX_U64">MAX_U64</a>;<br /><b>aborts_if</b> <a href="account.md#0x1_account">account</a>.guid_creation_num &#43; 1 &gt;&#61; <a href="account.md#0x1_account_MAX_GUID_CREATION_NUM">MAX_GUID_CREATION_NUM</a>;<br />&#125;<br /></code></pre>



<a id="@Specification_1_register_coin"></a>

### Function `register_coin`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="account.md#0x1_account_register_coin">register_coin</a>&lt;CoinType&gt;(account_addr: <b>address</b>)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /><b>aborts_if</b> !<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_spec_is_struct">type_info::spec_is_struct</a>&lt;CoinType&gt;();<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_addr);<br /></code></pre>



<a id="@Specification_1_create_signer_with_capability"></a>

### Function `create_signer_with_capability`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_signer_with_capability">create_signer_with_capability</a>(capability: &amp;<a href="account.md#0x1_account_SignerCapability">account::SignerCapability</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a><br /></code></pre>




<pre><code><b>let</b> addr &#61; capability.<a href="account.md#0x1_account">account</a>;<br /><b>ensures</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(result) &#61;&#61; addr;<br /></code></pre>




<a id="0x1_account_CreateResourceAccountAbortsIf"></a>


<pre><code><b>schema</b> <a href="account.md#0x1_account_CreateResourceAccountAbortsIf">CreateResourceAccountAbortsIf</a> &#123;<br />resource_addr: <b>address</b>;<br /><b>let</b> <a href="account.md#0x1_account">account</a> &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(resource_addr);<br /><b>aborts_if</b> len(<a href="account.md#0x1_account">account</a>.signer_capability_offer.for.vec) !&#61; 0;<br /><b>aborts_if</b> <a href="account.md#0x1_account">account</a>.sequence_number !&#61; 0;<br />&#125;<br /></code></pre>



<a id="@Specification_1_verify_signed_message"></a>

### Function `verify_signed_message`


<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_verify_signed_message">verify_signed_message</a>&lt;T: drop&gt;(<a href="account.md#0x1_account">account</a>: <b>address</b>, account_scheme: u8, account_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, signed_message_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, message: T)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>modifies</b> <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>);<br /><b>let</b> account_resource &#61; <b>global</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(<a href="account.md#0x1_account">account</a>);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: account_public_key &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_spec_public_key_bytes_to_authentication_key">ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key);<br />    account_resource.authentication_key !&#61; expected_auth_key<br />&#125;);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewUnvalidatedPublicKeyFromBytesAbortsIf">multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf</a> &#123; bytes: account_public_key &#125;;<br /><b>aborts_if</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &amp;&amp; (&#123;<br />    <b>let</b> expected_auth_key &#61; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_spec_public_key_bytes_to_authentication_key">multi_ed25519::spec_public_key_bytes_to_authentication_key</a>(account_public_key);<br />    account_resource.authentication_key !&#61; expected_auth_key<br />&#125;);<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_NewSignatureFromBytesAbortsIf">ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: signed_message_bytes &#125;;<br /><b>include</b> account_scheme &#61;&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a> &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/multi_ed25519.md#0x1_multi_ed25519_NewSignatureFromBytesAbortsIf">multi_ed25519::NewSignatureFromBytesAbortsIf</a> &#123; bytes: signed_message_bytes &#125;;<br /><b>aborts_if</b> account_scheme !&#61; <a href="account.md#0x1_account_ED25519_SCHEME">ED25519_SCHEME</a> &amp;&amp; account_scheme !&#61; <a href="account.md#0x1_account_MULTI_ED25519_SCHEME">MULTI_ED25519_SCHEME</a>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
