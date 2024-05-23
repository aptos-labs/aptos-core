
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


<pre><code>use 0x1::bcs;<br/>use 0x1::chain_id;<br/>use 0x1::create_signer;<br/>use 0x1::ed25519;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::from_bcs;<br/>use 0x1::guid;<br/>use 0x1::hash;<br/>use 0x1::multi_ed25519;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::system_addresses;<br/>use 0x1::table;<br/>use 0x1::type_info;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_account_KeyRotation"></a>

## Struct `KeyRotation`



<pre><code>&#35;[event]<br/>struct KeyRotation has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>old_authentication_key: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_authentication_key: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_Account"></a>

## Resource `Account`

Resource representing an account.


<pre><code>struct Account has store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>authentication_key: vector&lt;u8&gt;</code>
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
<code>coin_register_events: event::EventHandle&lt;account::CoinRegisterEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>key_rotation_events: event::EventHandle&lt;account::KeyRotationEvent&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>rotation_capability_offer: account::CapabilityOffer&lt;account::RotationCapability&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>signer_capability_offer: account::CapabilityOffer&lt;account::SignerCapability&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_KeyRotationEvent"></a>

## Struct `KeyRotationEvent`



<pre><code>struct KeyRotationEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>old_authentication_key: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>new_authentication_key: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_CoinRegisterEvent"></a>

## Struct `CoinRegisterEvent`



<pre><code>struct CoinRegisterEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>type_info: type_info::TypeInfo</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_CapabilityOffer"></a>

## Struct `CapabilityOffer`



<pre><code>struct CapabilityOffer&lt;T&gt; has store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>for: option::Option&lt;address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationCapability"></a>

## Struct `RotationCapability`



<pre><code>struct RotationCapability has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_SignerCapability"></a>

## Struct `SignerCapability`



<pre><code>struct SignerCapability has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_OriginatingAddress"></a>

## Resource `OriginatingAddress`

It is easy to fetch the authentication key of an address by simply reading it from the <code>Account</code> struct at that address.<br/> The table in this struct makes it possible to do a reverse lookup: it maps an authentication key, to the address of the account which has that authentication key set.<br/><br/> This mapping is needed when recovering wallets for accounts whose authentication key has been rotated.<br/><br/> For example, imagine a freshly&#45;created wallet with address <code>a</code> and thus also with authentication key <code>a</code>, derived from a PK <code>pk_a</code> with corresponding SK <code>sk_a</code>.<br/> It is easy to recover such a wallet given just the secret key <code>sk_a</code>, since the PK can be derived from the SK, the authentication key can then be derived from the PK, and the address equals the authentication key (since there was no key rotation).<br/><br/> However, if such a wallet rotates its authentication key to <code>b</code> derived from a different PK <code>pk_b</code> with SK <code>sk_b</code>, how would account recovery work?<br/> The recovered address would no longer be &apos;a&apos;; it would be <code>b</code>, which is incorrect.<br/> This struct solves this problem by mapping the new authentication key <code>b</code> to the original address <code>a</code> and thus helping the wallet software during recovery find the correct address.


<pre><code>struct OriginatingAddress has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>address_map: table::Table&lt;address, address&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationProofChallenge"></a>

## Struct `RotationProofChallenge`

This structs stores the challenge message that should be signed during key rotation. First, this struct is<br/> signed by the account owner&apos;s current public key, which proves possession of a capability to rotate the key.<br/> Second, this struct is signed by the new public key that the account owner wants to rotate to, which proves<br/> knowledge of this new public key&apos;s associated secret key. These two signatures cannot be replayed in another<br/> context because they include the TXN&apos;s unique sequence number.


<pre><code>struct RotationProofChallenge has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>originator: address</code>
</dt>
<dd>

</dd>
<dt>
<code>current_auth_key: address</code>
</dt>
<dd>

</dd>
<dt>
<code>new_public_key: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationCapabilityOfferProofChallenge"></a>

## Struct `RotationCapabilityOfferProofChallenge`

Deprecated struct &#45; newest version is <code>RotationCapabilityOfferProofChallengeV2</code>


<pre><code>struct RotationCapabilityOfferProofChallenge has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_SignerCapabilityOfferProofChallenge"></a>

## Struct `SignerCapabilityOfferProofChallenge`

Deprecated struct &#45; newest version is <code>SignerCapabilityOfferProofChallengeV2</code>


<pre><code>struct SignerCapabilityOfferProofChallenge has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationCapabilityOfferProofChallengeV2"></a>

## Struct `RotationCapabilityOfferProofChallengeV2`

This struct stores the challenge message that should be signed by the source account, when the source account<br/> is delegating its rotation capability to the <code>recipient_address</code>.<br/> This V2 struct adds the <code>chain_id</code> and <code>source_address</code> to the challenge message, which prevents replaying the challenge message.


<pre><code>struct RotationCapabilityOfferProofChallengeV2 has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>chain_id: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>source_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_SignerCapabilityOfferProofChallengeV2"></a>

## Struct `SignerCapabilityOfferProofChallengeV2`



<pre><code>struct SignerCapabilityOfferProofChallengeV2 has copy, drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sequence_number: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>source_address: address</code>
</dt>
<dd>

</dd>
<dt>
<code>recipient_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_account_MAX_U64"></a>



<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;<br/></code></pre>



<a id="0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME"></a>

Scheme identifier used when hashing an account&apos;s address together with a seed to derive the address (not the<br/> authentication key) of a resource account. This is an abuse of the notion of a scheme identifier which, for now,<br/> serves to domain separate hashes used to derive resource account addresses from hashes used to derive<br/> authentication keys. Without such separation, an adversary could create (and get a signer for) a resource account<br/> whose address matches an existing address of a MultiEd25519 wallet.


<pre><code>const DERIVE_RESOURCE_ACCOUNT_SCHEME: u8 &#61; 255;<br/></code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_EXISTS"></a>

Account already exists


<pre><code>const EACCOUNT_ALREADY_EXISTS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_USED"></a>

An attempt to create a resource account on an account that has a committed transaction


<pre><code>const EACCOUNT_ALREADY_USED: u64 &#61; 16;<br/></code></pre>



<a id="0x1_account_EACCOUNT_DOES_NOT_EXIST"></a>

Account does not exist


<pre><code>const EACCOUNT_DOES_NOT_EXIST: u64 &#61; 2;<br/></code></pre>



<a id="0x1_account_ECANNOT_RESERVED_ADDRESS"></a>

Cannot create account because address is reserved


<pre><code>const ECANNOT_RESERVED_ADDRESS: u64 &#61; 5;<br/></code></pre>



<a id="0x1_account_ED25519_SCHEME"></a>

Scheme identifier for Ed25519 signatures used to derive authentication keys for Ed25519 public keys.


<pre><code>const ED25519_SCHEME: u8 &#61; 0;<br/></code></pre>



<a id="0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM"></a>



<pre><code>const EEXCEEDED_MAX_GUID_CREATION_NUM: u64 &#61; 20;<br/></code></pre>



<a id="0x1_account_EINVALID_ACCEPT_ROTATION_CAPABILITY"></a>

The caller does not have a valid rotation capability offer from the other account


<pre><code>const EINVALID_ACCEPT_ROTATION_CAPABILITY: u64 &#61; 10;<br/></code></pre>



<a id="0x1_account_EINVALID_ORIGINATING_ADDRESS"></a>

Abort the transaction if the expected originating address is different from the originating address on&#45;chain


<pre><code>const EINVALID_ORIGINATING_ADDRESS: u64 &#61; 13;<br/></code></pre>



<a id="0x1_account_EINVALID_PROOF_OF_KNOWLEDGE"></a>

Specified proof of knowledge required to prove ownership of a public key is invalid


<pre><code>const EINVALID_PROOF_OF_KNOWLEDGE: u64 &#61; 8;<br/></code></pre>



<a id="0x1_account_EINVALID_SCHEME"></a>

Specified scheme required to proceed with the smart contract operation &#45; can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1)


<pre><code>const EINVALID_SCHEME: u64 &#61; 12;<br/></code></pre>



<a id="0x1_account_EMALFORMED_AUTHENTICATION_KEY"></a>

The provided authentication key has an invalid length


<pre><code>const EMALFORMED_AUTHENTICATION_KEY: u64 &#61; 4;<br/></code></pre>



<a id="0x1_account_ENO_CAPABILITY"></a>

The caller does not have a digital&#45;signature&#45;based capability to call this function


<pre><code>const ENO_CAPABILITY: u64 &#61; 9;<br/></code></pre>



<a id="0x1_account_ENO_SIGNER_CAPABILITY_OFFERED"></a>



<pre><code>const ENO_SIGNER_CAPABILITY_OFFERED: u64 &#61; 19;<br/></code></pre>



<a id="0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER"></a>

The specified rotation capablity offer does not exist at the specified offerer address


<pre><code>const ENO_SUCH_ROTATION_CAPABILITY_OFFER: u64 &#61; 18;<br/></code></pre>



<a id="0x1_account_ENO_SUCH_SIGNER_CAPABILITY"></a>

The signer capability offer doesn&apos;t exist at the given address


<pre><code>const ENO_SUCH_SIGNER_CAPABILITY: u64 &#61; 14;<br/></code></pre>



<a id="0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS"></a>

Address to create is not a valid reserved address for Aptos framework


<pre><code>const ENO_VALID_FRAMEWORK_RESERVED_ADDRESS: u64 &#61; 11;<br/></code></pre>



<a id="0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST"></a>

Offerer address doesn&apos;t exist


<pre><code>const EOFFERER_ADDRESS_DOES_NOT_EXIST: u64 &#61; 17;<br/></code></pre>



<a id="0x1_account_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code>const EOUT_OF_GAS: u64 &#61; 6;<br/></code></pre>



<a id="0x1_account_ERESOURCE_ACCCOUNT_EXISTS"></a>

An attempt to create a resource account on a claimed account


<pre><code>const ERESOURCE_ACCCOUNT_EXISTS: u64 &#61; 15;<br/></code></pre>



<a id="0x1_account_ESEQUENCE_NUMBER_TOO_BIG"></a>

Sequence number exceeds the maximum value for a u64


<pre><code>const ESEQUENCE_NUMBER_TOO_BIG: u64 &#61; 3;<br/></code></pre>



<a id="0x1_account_EWRONG_CURRENT_PUBLIC_KEY"></a>

Specified current public key is not correct


<pre><code>const EWRONG_CURRENT_PUBLIC_KEY: u64 &#61; 7;<br/></code></pre>



<a id="0x1_account_MAX_GUID_CREATION_NUM"></a>

Explicitly separate the GUID space between Object and Account to prevent accidental overlap.


<pre><code>const MAX_GUID_CREATION_NUM: u64 &#61; 1125899906842624;<br/></code></pre>



<a id="0x1_account_MULTI_ED25519_SCHEME"></a>

Scheme identifier for MultiEd25519 signatures used to derive authentication keys for MultiEd25519 public keys.


<pre><code>const MULTI_ED25519_SCHEME: u8 &#61; 1;<br/></code></pre>



<a id="0x1_account_ZERO_AUTH_KEY"></a>



<pre><code>const ZERO_AUTH_KEY: vector&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];<br/></code></pre>



<a id="0x1_account_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    move_to(aptos_framework, OriginatingAddress &#123;<br/>        address_map: table::new(),<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_account_if_does_not_exist"></a>

## Function `create_account_if_does_not_exist`



<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address)
>>>>>>> 13c50e058f (support mdx)
=======
<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
>>>>>>> 33ab75447f (back to default)
</code></pre>
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address)<br/></code></pre>
>>>>>>> fa7dac41c8 (mdx docs)



<details>
<summary>Implementation</summary>


<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>) {
=======
<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>) {
>>>>>>> 33ab75447f (back to default)
    <b>if</b> (!<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address)) {
        <a href="account.md#0x1_account_create_account">create_account</a>(account_address);
    }
}
<<<<<<< HEAD
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address) &#123;
    if (!exists&lt;Account&gt;(account_address)) &#123;
        create_account(account_address);
    &#125;
&#125;
>>>>>>> 13c50e058f (support mdx)
=======
>>>>>>> 33ab75447f (back to default)
</code></pre>
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address) &#123;<br/>    if (!exists&lt;Account&gt;(account_address)) &#123;<br/>        create_account(account_address);<br/>    &#125;<br/>&#125;<br/></code></pre>
>>>>>>> fa7dac41c8 (mdx docs)



</details>

<a id="0x1_account_create_account"></a>

## Function `create_account`

Publishes a new <code>Account</code> resource under <code>new_address</code>. A signer representing <code>new_address</code><br/> is returned. This way, the caller of this function can publish additional resources under<br/> <code>new_address</code>.


<pre><code>public(friend) fun create_account(new_address: address): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_account(new_address: address): signer &#123;<br/>    // there cannot be an Account resource under new_addr already.<br/>    assert!(!exists&lt;Account&gt;(new_address), error::already_exists(EACCOUNT_ALREADY_EXISTS));<br/><br/>    // NOTE: @core_resources gets created via a `create_account` call, so we do not include it below.<br/>    assert!(<br/>        new_address !&#61; @vm_reserved &amp;&amp; new_address !&#61; @aptos_framework &amp;&amp; new_address !&#61; @aptos_token,<br/>        error::invalid_argument(ECANNOT_RESERVED_ADDRESS)<br/>    );<br/><br/>    create_account_unchecked(new_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_account_unchecked"></a>

## Function `create_account_unchecked`



<pre><code>fun create_account_unchecked(new_address: address): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_account_unchecked(new_address: address): signer &#123;<br/>    let new_account &#61; create_signer(new_address);<br/>    let authentication_key &#61; bcs::to_bytes(&amp;new_address);<br/>    assert!(<br/>        vector::length(&amp;authentication_key) &#61;&#61; 32,<br/>        error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)<br/>    );<br/><br/>    let guid_creation_num &#61; 0;<br/><br/>    let guid_for_coin &#61; guid::create(new_address, &amp;mut guid_creation_num);<br/>    let coin_register_events &#61; event::new_event_handle&lt;CoinRegisterEvent&gt;(guid_for_coin);<br/><br/>    let guid_for_rotation &#61; guid::create(new_address, &amp;mut guid_creation_num);<br/>    let key_rotation_events &#61; event::new_event_handle&lt;KeyRotationEvent&gt;(guid_for_rotation);<br/><br/>    move_to(<br/>        &amp;new_account,<br/>        Account &#123;<br/>            authentication_key,<br/>            sequence_number: 0,<br/>            guid_creation_num,<br/>            coin_register_events,<br/>            key_rotation_events,<br/>            rotation_capability_offer: CapabilityOffer &#123; for: option::none() &#125;,<br/>            signer_capability_offer: CapabilityOffer &#123; for: option::none() &#125;,<br/>        &#125;<br/>    );<br/><br/>    new_account<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_exists_at"></a>

## Function `exists_at`



<pre><code>&#35;[view]<br/>public fun exists_at(addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun exists_at(addr: address): bool &#123;<br/>    exists&lt;Account&gt;(addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_get_guid_next_creation_num"></a>

## Function `get_guid_next_creation_num`



<pre><code>&#35;[view]<br/>public fun get_guid_next_creation_num(addr: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_guid_next_creation_num(addr: address): u64 acquires Account &#123;<br/>    borrow_global&lt;Account&gt;(addr).guid_creation_num<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_get_sequence_number"></a>

## Function `get_sequence_number`



<pre><code>&#35;[view]<br/>public fun get_sequence_number(addr: address): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_sequence_number(addr: address): u64 acquires Account &#123;<br/>    borrow_global&lt;Account&gt;(addr).sequence_number<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_increment_sequence_number"></a>

## Function `increment_sequence_number`



<pre><code>public(friend) fun increment_sequence_number(addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun increment_sequence_number(addr: address) acquires Account &#123;<br/>    let sequence_number &#61; &amp;mut borrow_global_mut&lt;Account&gt;(addr).sequence_number;<br/><br/>    assert!(<br/>        (&#42;sequence_number as u128) &lt; MAX_U64,<br/>        error::out_of_range(ESEQUENCE_NUMBER_TOO_BIG)<br/>    );<br/><br/>    &#42;sequence_number &#61; &#42;sequence_number &#43; 1;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_get_authentication_key"></a>

## Function `get_authentication_key`



<pre><code>&#35;[view]<br/>public fun get_authentication_key(addr: address): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_authentication_key(addr: address): vector&lt;u8&gt; acquires Account &#123;<br/>    borrow_global&lt;Account&gt;(addr).authentication_key<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_internal"></a>

## Function `rotate_authentication_key_internal`

This function is used to rotate a resource account&apos;s authentication key to <code>new_auth_key</code>. This is done in<br/> many contexts:<br/> 1. During normal key rotation via <code>rotate_authentication_key</code> or <code>rotate_authentication_key_call</code><br/> 2. During resource account initialization so that no private key can control the resource account<br/> 3. During multisig_v2 account creation


<pre><code>public(friend) fun rotate_authentication_key_internal(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun rotate_authentication_key_internal(account: &amp;signer, new_auth_key: vector&lt;u8&gt;) acquires Account &#123;<br/>    let addr &#61; signer::address_of(account);<br/>    assert!(exists_at(addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));<br/>    assert!(<br/>        vector::length(&amp;new_auth_key) &#61;&#61; 32,<br/>        error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)<br/>    );<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);<br/>    account_resource.authentication_key &#61; new_auth_key;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_call"></a>

## Function `rotate_authentication_key_call`

Private entry function for key rotation that allows the signer to update their authentication key.<br/> Note that this does not update the <code>OriginatingAddress</code> table because the <code>new_auth_key</code> is not &quot;verified&quot;: it<br/> does not come with a proof&#45;of&#45;knowledge of the underlying SK. Nonetheless, we need this functionality due to<br/> the introduction of non&#45;standard key algorithms, such as passkeys, which cannot produce proofs&#45;of&#45;knowledge in<br/> the format expected in <code>rotate_authentication_key</code>.


<pre><code>entry fun rotate_authentication_key_call(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun rotate_authentication_key_call(account: &amp;signer, new_auth_key: vector&lt;u8&gt;) acquires Account &#123;<br/>    rotate_authentication_key_internal(account, new_auth_key);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key"></a>

## Function `rotate_authentication_key`

Generic authentication key rotation function that allows the user to rotate their authentication key from any scheme to any scheme.<br/> To authorize the rotation, we need two signatures:<br/> &#45; the first signature <code>cap_rotate_key</code> refers to the signature by the account owner&apos;s current key on a valid <code>RotationProofChallenge</code>,<br/> demonstrating that the user intends to and has the capability to rotate the authentication key of this account;<br/> &#45; the second signature <code>cap_update_table</code> refers to the signature by the new key (that the account owner wants to rotate to) on a<br/> valid <code>RotationProofChallenge</code>, demonstrating that the user owns the new private key, and has the authority to update the<br/> <code>OriginatingAddress</code> map with the new address mapping <code>&lt;new_address, originating_address&gt;</code>.<br/> To verify these two signatures, we need their corresponding public key and public key scheme: we use <code>from_scheme</code> and <code>from_public_key_bytes</code><br/> to verify <code>cap_rotate_key</code>, and <code>to_scheme</code> and <code>to_public_key_bytes</code> to verify <code>cap_update_table</code>.<br/> A scheme of 0 refers to an Ed25519 key and a scheme of 1 refers to Multi&#45;Ed25519 keys.<br/> <code>originating address</code> refers to an account&apos;s original/first address.<br/><br/> Here is an example attack if we don&apos;t ask for the second signature <code>cap_update_table</code>:<br/> Alice has rotated her account <code>addr_a</code> to <code>new_addr_a</code>. As a result, the following entry is created, to help Alice when recovering her wallet:<br/> <code>OriginatingAddress[new_addr_a]</code> &#45;&gt; <code>addr_a</code><br/> Alice has had bad day: her laptop blew up and she needs to reset her account on a new one.<br/> (Fortunately, she still has her secret key <code>new_sk_a</code> associated with her new address <code>new_addr_a</code>, so she can do this.)<br/><br/> But Bob likes to mess with Alice.<br/> Bob creates an account <code>addr_b</code> and maliciously rotates it to Alice&apos;s new address <code>new_addr_a</code>. Since we are no longer checking a PoK,<br/> Bob can easily do this.<br/><br/> Now, the table will be updated to make Alice&apos;s new address point to Bob&apos;s address: <code>OriginatingAddress[new_addr_a]</code> &#45;&gt; <code>addr_b</code>.<br/> When Alice recovers her account, her wallet will display the attacker&apos;s address (Bob&apos;s) <code>addr_b</code> as her address.<br/> Now Alice will give <code>addr_b</code> to everyone to pay her, but the money will go to Bob.<br/><br/> Because we ask for a valid <code>cap_update_table</code>, this kind of attack is not possible. Bob would not have the secret key of Alice&apos;s address<br/> to rotate his address to Alice&apos;s address in the first place.


<pre><code>public entry fun rotate_authentication_key(account: &amp;signer, from_scheme: u8, from_public_key_bytes: vector&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: vector&lt;u8&gt;, cap_rotate_key: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun rotate_authentication_key(<br/>    account: &amp;signer,<br/>    from_scheme: u8,<br/>    from_public_key_bytes: vector&lt;u8&gt;,<br/>    to_scheme: u8,<br/>    to_public_key_bytes: vector&lt;u8&gt;,<br/>    cap_rotate_key: vector&lt;u8&gt;,<br/>    cap_update_table: vector&lt;u8&gt;,<br/>) acquires Account, OriginatingAddress &#123;<br/>    let addr &#61; signer::address_of(account);<br/>    assert!(exists_at(addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);<br/><br/>    // Verify the given `from_public_key_bytes` matches this account&apos;s current authentication key.<br/>    if (from_scheme &#61;&#61; ED25519_SCHEME) &#123;<br/>        let from_pk &#61; ed25519::new_unvalidated_public_key_from_bytes(from_public_key_bytes);<br/>        let from_auth_key &#61; ed25519::unvalidated_public_key_to_authentication_key(&amp;from_pk);<br/>        assert!(<br/>            account_resource.authentication_key &#61;&#61; from_auth_key,<br/>            error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY)<br/>        );<br/>    &#125; else if (from_scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;<br/>        let from_pk &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(from_public_key_bytes);<br/>        let from_auth_key &#61; multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;from_pk);<br/>        assert!(<br/>            account_resource.authentication_key &#61;&#61; from_auth_key,<br/>            error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY)<br/>        );<br/>    &#125; else &#123;<br/>        abort error::invalid_argument(EINVALID_SCHEME)<br/>    &#125;;<br/><br/>    // Construct a valid `RotationProofChallenge` that `cap_rotate_key` and `cap_update_table` will validate against.<br/>    let curr_auth_key_as_address &#61; from_bcs::to_address(account_resource.authentication_key);<br/>    let challenge &#61; RotationProofChallenge &#123;<br/>        sequence_number: account_resource.sequence_number,<br/>        originator: addr,<br/>        current_auth_key: curr_auth_key_as_address,<br/>        new_public_key: to_public_key_bytes,<br/>    &#125;;<br/><br/>    // Assert the challenges signed by the current and new keys are valid<br/>    assert_valid_rotation_proof_signature_and_get_auth_key(<br/>        from_scheme,<br/>        from_public_key_bytes,<br/>        cap_rotate_key,<br/>        &amp;challenge<br/>    );<br/>    let new_auth_key &#61; assert_valid_rotation_proof_signature_and_get_auth_key(<br/>        to_scheme,<br/>        to_public_key_bytes,<br/>        cap_update_table,<br/>        &amp;challenge<br/>    );<br/><br/>    // Update the `OriginatingAddress` table.<br/>    update_auth_key_and_originating_address_table(addr, account_resource, new_auth_key);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_with_rotation_capability"></a>

## Function `rotate_authentication_key_with_rotation_capability`



<pre><code>public entry fun rotate_authentication_key_with_rotation_capability(delegate_signer: &amp;signer, rotation_cap_offerer_address: address, new_scheme: u8, new_public_key_bytes: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun rotate_authentication_key_with_rotation_capability(<br/>    delegate_signer: &amp;signer,<br/>    rotation_cap_offerer_address: address,<br/>    new_scheme: u8,<br/>    new_public_key_bytes: vector&lt;u8&gt;,<br/>    cap_update_table: vector&lt;u8&gt;<br/>) acquires Account, OriginatingAddress &#123;<br/>    assert!(exists_at(rotation_cap_offerer_address), error::not_found(EOFFERER_ADDRESS_DOES_NOT_EXIST));<br/><br/>    // Check that there exists a rotation capability offer at the offerer&apos;s account resource for the delegate.<br/>    let delegate_address &#61; signer::address_of(delegate_signer);<br/>    let offerer_account_resource &#61; borrow_global&lt;Account&gt;(rotation_cap_offerer_address);<br/>    assert!(<br/>        option::contains(&amp;offerer_account_resource.rotation_capability_offer.for, &amp;delegate_address),<br/>        error::not_found(ENO_SUCH_ROTATION_CAPABILITY_OFFER)<br/>    );<br/><br/>    let curr_auth_key &#61; from_bcs::to_address(offerer_account_resource.authentication_key);<br/>    let challenge &#61; RotationProofChallenge &#123;<br/>        sequence_number: get_sequence_number(delegate_address),<br/>        originator: rotation_cap_offerer_address,<br/>        current_auth_key: curr_auth_key,<br/>        new_public_key: new_public_key_bytes,<br/>    &#125;;<br/><br/>    // Verifies that the `RotationProofChallenge` from above is signed under the new public key that we are rotating to.        l<br/>    let new_auth_key &#61; assert_valid_rotation_proof_signature_and_get_auth_key(<br/>        new_scheme,<br/>        new_public_key_bytes,<br/>        cap_update_table,<br/>        &amp;challenge<br/>    );<br/><br/>    // Update the `OriginatingAddress` table, so we can find the originating address using the new address.<br/>    let offerer_account_resource &#61; borrow_global_mut&lt;Account&gt;(rotation_cap_offerer_address);<br/>    update_auth_key_and_originating_address_table(<br/>        rotation_cap_offerer_address,<br/>        offerer_account_resource,<br/>        new_auth_key<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_offer_rotation_capability"></a>

## Function `offer_rotation_capability`

Offers rotation capability on behalf of <code>account</code> to the account at address <code>recipient_address</code>.<br/> An account can delegate its rotation capability to only one other address at one time. If the account<br/> has an existing rotation capability offer, calling this function will update the rotation capability offer with<br/> the new <code>recipient_address</code>.<br/> Here, <code>rotation_capability_sig_bytes</code> signature indicates that this key rotation is authorized by the account owner,<br/> and prevents the classic &quot;time&#45;of&#45;check time&#45;of&#45;use&quot; attack.<br/> For example, users usually rely on what the wallet displays to them as the transaction&apos;s outcome. Consider a contract that with 50% probability
(based on the current timestamp in Move), rotates somebody&apos;s key. The wallet might be unlucky and get an outcome where nothing is rotated,<br/> incorrectly telling the user nothing bad will happen. But when the transaction actually gets executed, the attacker gets lucky and<br/> the execution path triggers the account key rotation.<br/> We prevent such attacks by asking for this extra signature authorizing the key rotation.<br/><br/> @param rotation_capability_sig_bytes is the signature by the account owner&apos;s key on <code>RotationCapabilityOfferProofChallengeV2</code>.<br/> @param account_scheme is the scheme of the account (ed25519 or multi_ed25519).<br/> @param account_public_key_bytes is the public key of the account owner.<br/> @param recipient_address is the address of the recipient of the rotation capability &#45; note that if there&apos;s an existing rotation capability<br/> offer, calling this function will replace the previous <code>recipient_address</code> upon successful verification.


<pre><code>public entry fun offer_rotation_capability(account: &amp;signer, rotation_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun offer_rotation_capability(<br/>    account: &amp;signer,<br/>    rotation_capability_sig_bytes: vector&lt;u8&gt;,<br/>    account_scheme: u8,<br/>    account_public_key_bytes: vector&lt;u8&gt;,<br/>    recipient_address: address,<br/>) acquires Account &#123;<br/>    let addr &#61; signer::address_of(account);<br/>    assert!(exists_at(recipient_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));<br/><br/>    // proof that this account intends to delegate its rotation capability to another account<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);<br/>    let proof_challenge &#61; RotationCapabilityOfferProofChallengeV2 &#123;<br/>        chain_id: chain_id::get(),<br/>        sequence_number: account_resource.sequence_number,<br/>        source_address: addr,<br/>        recipient_address,<br/>    &#125;;<br/><br/>    // verify the signature on `RotationCapabilityOfferProofChallengeV2` by the account owner<br/>    if (account_scheme &#61;&#61; ED25519_SCHEME) &#123;<br/>        let pubkey &#61; ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);<br/>        let expected_auth_key &#61; ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);<br/>        assert!(<br/>            account_resource.authentication_key &#61;&#61; expected_auth_key,<br/>            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY)<br/>        );<br/><br/>        let rotation_capability_sig &#61; ed25519::new_signature_from_bytes(rotation_capability_sig_bytes);<br/>        assert!(<br/>            ed25519::signature_verify_strict_t(&amp;rotation_capability_sig, &amp;pubkey, proof_challenge),<br/>            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)<br/>        );<br/>    &#125; else if (account_scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;<br/>        let pubkey &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);<br/>        let expected_auth_key &#61; multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);<br/>        assert!(<br/>            account_resource.authentication_key &#61;&#61; expected_auth_key,<br/>            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY)<br/>        );<br/><br/>        let rotation_capability_sig &#61; multi_ed25519::new_signature_from_bytes(rotation_capability_sig_bytes);<br/>        assert!(<br/>            multi_ed25519::signature_verify_strict_t(&amp;rotation_capability_sig, &amp;pubkey, proof_challenge),<br/>            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)<br/>        );<br/>    &#125; else &#123;<br/>        abort error::invalid_argument(EINVALID_SCHEME)<br/>    &#125;;<br/><br/>    // update the existing rotation capability offer or put in a new rotation capability offer for the current account<br/>    option::swap_or_fill(&amp;mut account_resource.rotation_capability_offer.for, recipient_address);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_is_rotation_capability_offered"></a>

## Function `is_rotation_capability_offered`

Returns true if the account at <code>account_addr</code> has a rotation capability offer.


<pre><code>&#35;[view]<br/>public fun is_rotation_capability_offered(account_addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_rotation_capability_offered(account_addr: address): bool acquires Account &#123;<br/>    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);<br/>    option::is_some(&amp;account_resource.rotation_capability_offer.for)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_get_rotation_capability_offer_for"></a>

## Function `get_rotation_capability_offer_for`

Returns the address of the account that has a rotation capability offer from the account at <code>account_addr</code>.


<pre><code>&#35;[view]<br/>public fun get_rotation_capability_offer_for(account_addr: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_rotation_capability_offer_for(account_addr: address): address acquires Account &#123;<br/>    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);<br/>    assert!(<br/>        option::is_some(&amp;account_resource.rotation_capability_offer.for),<br/>        error::not_found(ENO_SIGNER_CAPABILITY_OFFERED),<br/>    );<br/>    &#42;option::borrow(&amp;account_resource.rotation_capability_offer.for)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_revoke_rotation_capability"></a>

## Function `revoke_rotation_capability`

Revoke the rotation capability offer given to <code>to_be_revoked_recipient_address</code> from <code>account</code>


<pre><code>public entry fun revoke_rotation_capability(account: &amp;signer, to_be_revoked_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_rotation_capability(account: &amp;signer, to_be_revoked_address: address) acquires Account &#123;<br/>    assert!(exists_at(to_be_revoked_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));<br/>    let addr &#61; signer::address_of(account);<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);<br/>    assert!(<br/>        option::contains(&amp;account_resource.rotation_capability_offer.for, &amp;to_be_revoked_address),<br/>        error::not_found(ENO_SUCH_ROTATION_CAPABILITY_OFFER)<br/>    );<br/>    revoke_any_rotation_capability(account);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_revoke_any_rotation_capability"></a>

## Function `revoke_any_rotation_capability`

Revoke any rotation capability offer in the specified account.


<pre><code>public entry fun revoke_any_rotation_capability(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_any_rotation_capability(account: &amp;signer) acquires Account &#123;<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(signer::address_of(account));<br/>    option::extract(&amp;mut account_resource.rotation_capability_offer.for);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_offer_signer_capability"></a>

## Function `offer_signer_capability`

Offers signer capability on behalf of <code>account</code> to the account at address <code>recipient_address</code>.<br/> An account can delegate its signer capability to only one other address at one time.<br/> <code>signer_capability_key_bytes</code> is the <code>SignerCapabilityOfferProofChallengeV2</code> signed by the account owner&apos;s key<br/> <code>account_scheme</code> is the scheme of the account (ed25519 or multi_ed25519).<br/> <code>account_public_key_bytes</code> is the public key of the account owner.<br/> <code>recipient_address</code> is the address of the recipient of the signer capability &#45; note that if there&apos;s an existing<br/> <code>recipient_address</code> in the account owner&apos;s <code>SignerCapabilityOffer</code>, this will replace the<br/> previous <code>recipient_address</code> upon successful verification (the previous recipient will no longer have access<br/> to the account owner&apos;s signer capability).


<pre><code>public entry fun offer_signer_capability(account: &amp;signer, signer_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun offer_signer_capability(<br/>    account: &amp;signer,<br/>    signer_capability_sig_bytes: vector&lt;u8&gt;,<br/>    account_scheme: u8,<br/>    account_public_key_bytes: vector&lt;u8&gt;,<br/>    recipient_address: address<br/>) acquires Account &#123;<br/>    let source_address &#61; signer::address_of(account);<br/>    assert!(exists_at(recipient_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));<br/><br/>    // Proof that this account intends to delegate its signer capability to another account.<br/>    let proof_challenge &#61; SignerCapabilityOfferProofChallengeV2 &#123;<br/>        sequence_number: get_sequence_number(source_address),<br/>        source_address,<br/>        recipient_address,<br/>    &#125;;<br/>    verify_signed_message(<br/>        source_address, account_scheme, account_public_key_bytes, signer_capability_sig_bytes, proof_challenge);<br/><br/>    // Update the existing signer capability offer or put in a new signer capability offer for the recipient.<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(source_address);<br/>    option::swap_or_fill(&amp;mut account_resource.signer_capability_offer.for, recipient_address);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_is_signer_capability_offered"></a>

## Function `is_signer_capability_offered`

Returns true if the account at <code>account_addr</code> has a signer capability offer.


<pre><code>&#35;[view]<br/>public fun is_signer_capability_offered(account_addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_signer_capability_offered(account_addr: address): bool acquires Account &#123;<br/>    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);<br/>    option::is_some(&amp;account_resource.signer_capability_offer.for)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_get_signer_capability_offer_for"></a>

## Function `get_signer_capability_offer_for`

Returns the address of the account that has a signer capability offer from the account at <code>account_addr</code>.


<pre><code>&#35;[view]<br/>public fun get_signer_capability_offer_for(account_addr: address): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_signer_capability_offer_for(account_addr: address): address acquires Account &#123;<br/>    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);<br/>    assert!(<br/>        option::is_some(&amp;account_resource.signer_capability_offer.for),<br/>        error::not_found(ENO_SIGNER_CAPABILITY_OFFERED),<br/>    );<br/>    &#42;option::borrow(&amp;account_resource.signer_capability_offer.for)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_revoke_signer_capability"></a>

## Function `revoke_signer_capability`

Revoke the account owner&apos;s signer capability offer for <code>to_be_revoked_address</code> (i.e., the address that<br/> has a signer capability offer from <code>account</code> but will be revoked in this function).


<pre><code>public entry fun revoke_signer_capability(account: &amp;signer, to_be_revoked_address: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_signer_capability(account: &amp;signer, to_be_revoked_address: address) acquires Account &#123;<br/>    assert!(exists_at(to_be_revoked_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));<br/>    let addr &#61; signer::address_of(account);<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);<br/>    assert!(<br/>        option::contains(&amp;account_resource.signer_capability_offer.for, &amp;to_be_revoked_address),<br/>        error::not_found(ENO_SUCH_SIGNER_CAPABILITY)<br/>    );<br/>    revoke_any_signer_capability(account);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_revoke_any_signer_capability"></a>

## Function `revoke_any_signer_capability`

Revoke any signer capability offer in the specified account.


<pre><code>public entry fun revoke_any_signer_capability(account: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_any_signer_capability(account: &amp;signer) acquires Account &#123;<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(signer::address_of(account));<br/>    option::extract(&amp;mut account_resource.signer_capability_offer.for);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_authorized_signer"></a>

## Function `create_authorized_signer`

Return an authorized signer of the offerer, if there&apos;s an existing signer capability offer for <code>account</code><br/> at the offerer&apos;s address.


<pre><code>public fun create_authorized_signer(account: &amp;signer, offerer_address: address): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_authorized_signer(account: &amp;signer, offerer_address: address): signer acquires Account &#123;<br/>    assert!(exists_at(offerer_address), error::not_found(EOFFERER_ADDRESS_DOES_NOT_EXIST));<br/><br/>    // Check if there&apos;s an existing signer capability offer from the offerer.<br/>    let account_resource &#61; borrow_global&lt;Account&gt;(offerer_address);<br/>    let addr &#61; signer::address_of(account);<br/>    assert!(<br/>        option::contains(&amp;account_resource.signer_capability_offer.for, &amp;addr),<br/>        error::not_found(ENO_SUCH_SIGNER_CAPABILITY)<br/>    );<br/><br/>    create_signer(offerer_address)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

## Function `assert_valid_rotation_proof_signature_and_get_auth_key`

Helper functions for authentication key rotation.


<pre><code>fun assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector&lt;u8&gt;, signature: vector&lt;u8&gt;, challenge: &amp;account::RotationProofChallenge): vector&lt;u8&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_valid_rotation_proof_signature_and_get_auth_key(<br/>    scheme: u8,<br/>    public_key_bytes: vector&lt;u8&gt;,<br/>    signature: vector&lt;u8&gt;,<br/>    challenge: &amp;RotationProofChallenge<br/>): vector&lt;u8&gt; &#123;<br/>    if (scheme &#61;&#61; ED25519_SCHEME) &#123;<br/>        let pk &#61; ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);<br/>        let sig &#61; ed25519::new_signature_from_bytes(signature);<br/>        assert!(<br/>            ed25519::signature_verify_strict_t(&amp;sig, &amp;pk, &#42;challenge),<br/>            std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)<br/>        );<br/>        ed25519::unvalidated_public_key_to_authentication_key(&amp;pk)<br/>    &#125; else if (scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;<br/>        let pk &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);<br/>        let sig &#61; multi_ed25519::new_signature_from_bytes(signature);<br/>        assert!(<br/>            multi_ed25519::signature_verify_strict_t(&amp;sig, &amp;pk, &#42;challenge),<br/>            std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)<br/>        );<br/>        multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;pk)<br/>    &#125; else &#123;<br/>        abort error::invalid_argument(EINVALID_SCHEME)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_update_auth_key_and_originating_address_table"></a>

## Function `update_auth_key_and_originating_address_table`

Update the <code>OriginatingAddress</code> table, so that we can find the originating address using the latest address<br/> in the event of key recovery.


<pre><code>fun update_auth_key_and_originating_address_table(originating_addr: address, account_resource: &amp;mut account::Account, new_auth_key_vector: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_auth_key_and_originating_address_table(<br/>    originating_addr: address,<br/>    account_resource: &amp;mut Account,<br/>    new_auth_key_vector: vector&lt;u8&gt;,<br/>) acquires OriginatingAddress &#123;<br/>    let address_map &#61; &amp;mut borrow_global_mut&lt;OriginatingAddress&gt;(@aptos_framework).address_map;<br/>    let curr_auth_key &#61; from_bcs::to_address(account_resource.authentication_key);<br/><br/>    // Checks `OriginatingAddress[curr_auth_key]` is either unmapped, or mapped to `originating_address`.<br/>    // If it&apos;s mapped to the originating address, removes that mapping.<br/>    // Otherwise, abort if it&apos;s mapped to a different address.<br/>    if (table::contains(address_map, curr_auth_key)) &#123;<br/>        // If account_a with address_a is rotating its keypair from keypair_a to keypair_b, we expect<br/>        // the address of the account to stay the same, while its keypair updates to keypair_b.<br/>        // Here, by asserting that we&apos;re calling from the account with the originating address, we enforce<br/>        // the standard of keeping the same address and updating the keypair at the contract level.<br/>        // Without this assertion, the dapps could also update the account&apos;s address to address_b (the address that<br/>        // is programmatically related to keypaier_b) and update the keypair to keypair_b. This causes problems<br/>        // for interoperability because different dapps can implement this in different ways.<br/>        // If the account with address b calls this function with two valid signatures, it will abort at this step,<br/>        // because address b is not the account&apos;s originating address.<br/>        assert!(<br/>            originating_addr &#61;&#61; table::remove(address_map, curr_auth_key),<br/>            error::not_found(EINVALID_ORIGINATING_ADDRESS)<br/>        );<br/>    &#125;;<br/><br/>    // Set `OriginatingAddress[new_auth_key] &#61; originating_address`.<br/>    let new_auth_key &#61; from_bcs::to_address(new_auth_key_vector);<br/>    table::add(address_map, new_auth_key, originating_addr);<br/><br/>    if (std::features::module_event_migration_enabled()) &#123;<br/>        event::emit(KeyRotation &#123;<br/>            account: originating_addr,<br/>            old_authentication_key: account_resource.authentication_key,<br/>            new_authentication_key: new_auth_key_vector,<br/>        &#125;);<br/>    &#125;;<br/>    event::emit_event&lt;KeyRotationEvent&gt;(<br/>        &amp;mut account_resource.key_rotation_events,<br/>        KeyRotationEvent &#123;<br/>            old_authentication_key: account_resource.authentication_key,<br/>            new_authentication_key: new_auth_key_vector,<br/>        &#125;<br/>    );<br/><br/>    // Update the account resource&apos;s authentication key.<br/>    account_resource.authentication_key &#61; new_auth_key_vector;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_resource_address"></a>

## Function `create_resource_address`

Basic account creation methods.<br/> This is a helper function to compute resource addresses. Computation of the address<br/> involves the use of a cryptographic hash operation and should be use thoughtfully.


<pre><code>public fun create_resource_address(source: &amp;address, seed: vector&lt;u8&gt;): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_resource_address(source: &amp;address, seed: vector&lt;u8&gt;): address &#123;<br/>    let bytes &#61; bcs::to_bytes(source);<br/>    vector::append(&amp;mut bytes, seed);<br/>    vector::push_back(&amp;mut bytes, DERIVE_RESOURCE_ACCOUNT_SCHEME);<br/>    from_bcs::to_address(hash::sha3_256(bytes))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_resource_account"></a>

## Function `create_resource_account`

A resource account is used to manage resources independent of an account managed by a user.<br/> In Aptos a resource account is created based upon the sha3 256 of the source&apos;s address and additional seed data.<br/> A resource account can only be created once, this is designated by setting the<br/> <code>Account::signer_capability_offer::for</code> to the address of the resource account. While an entity may call<br/> <code>create_account</code> to attempt to claim an account ahead of the creation of a resource account, if found Aptos will<br/> transition ownership of the account over to the resource account. This is done by validating that the account has<br/> yet to execute any transactions and that the <code>Account::signer_capability_offer::for</code> is none. The probability of a<br/> collision where someone has legitimately produced a private key that maps to a resource account address is less<br/> than <code>(1/2)^(256)</code>.


<pre><code>public fun create_resource_account(source: &amp;signer, seed: vector&lt;u8&gt;): (signer, account::SignerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_resource_account(source: &amp;signer, seed: vector&lt;u8&gt;): (signer, SignerCapability) acquires Account &#123;<br/>    let resource_addr &#61; create_resource_address(&amp;signer::address_of(source), seed);<br/>    let resource &#61; if (exists_at(resource_addr)) &#123;<br/>        let account &#61; borrow_global&lt;Account&gt;(resource_addr);<br/>        assert!(<br/>            option::is_none(&amp;account.signer_capability_offer.for),<br/>            error::already_exists(ERESOURCE_ACCCOUNT_EXISTS),<br/>        );<br/>        assert!(<br/>            account.sequence_number &#61;&#61; 0,<br/>            error::invalid_state(EACCOUNT_ALREADY_USED),<br/>        );<br/>        create_signer(resource_addr)<br/>    &#125; else &#123;<br/>        create_account_unchecked(resource_addr)<br/>    &#125;;<br/><br/>    // By default, only the SignerCapability should have control over the resource account and not the auth key.<br/>    // If the source account wants direct control via auth key, they would need to explicitly rotate the auth key<br/>    // of the resource account using the SignerCapability.<br/>    rotate_authentication_key_internal(&amp;resource, ZERO_AUTH_KEY);<br/><br/>    let account &#61; borrow_global_mut&lt;Account&gt;(resource_addr);<br/>    account.signer_capability_offer.for &#61; option::some(resource_addr);<br/>    let signer_cap &#61; SignerCapability &#123; account: resource_addr &#125;;<br/>    (resource, signer_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_framework_reserved_account"></a>

## Function `create_framework_reserved_account`

create the account for system reserved addresses


<pre><code>public(friend) fun create_framework_reserved_account(addr: address): (signer, account::SignerCapability)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_framework_reserved_account(addr: address): (signer, SignerCapability) &#123;<br/>    assert!(<br/>        addr &#61;&#61; @0x1 &#124;&#124;<br/>            addr &#61;&#61; @0x2 &#124;&#124;<br/>            addr &#61;&#61; @0x3 &#124;&#124;<br/>            addr &#61;&#61; @0x4 &#124;&#124;<br/>            addr &#61;&#61; @0x5 &#124;&#124;<br/>            addr &#61;&#61; @0x6 &#124;&#124;<br/>            addr &#61;&#61; @0x7 &#124;&#124;<br/>            addr &#61;&#61; @0x8 &#124;&#124;<br/>            addr &#61;&#61; @0x9 &#124;&#124;<br/>            addr &#61;&#61; @0xa,<br/>        error::permission_denied(ENO_VALID_FRAMEWORK_RESERVED_ADDRESS),<br/>    );<br/>    let signer &#61; create_account_unchecked(addr);<br/>    let signer_cap &#61; SignerCapability &#123; account: addr &#125;;<br/>    (signer, signer_cap)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_guid"></a>

## Function `create_guid`

GUID management methods.


<pre><code>public fun create_guid(account_signer: &amp;signer): guid::GUID<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_guid(account_signer: &amp;signer): guid::GUID acquires Account &#123;<br/>    let addr &#61; signer::address_of(account_signer);<br/>    let account &#61; borrow_global_mut&lt;Account&gt;(addr);<br/>    let guid &#61; guid::create(addr, &amp;mut account.guid_creation_num);<br/>    assert!(<br/>        account.guid_creation_num &lt; MAX_GUID_CREATION_NUM,<br/>        error::out_of_range(EEXCEEDED_MAX_GUID_CREATION_NUM),<br/>    );<br/>    guid<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_new_event_handle"></a>

## Function `new_event_handle`

GUID management methods.


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(account: &amp;signer): event::EventHandle&lt;T&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_event_handle&lt;T: drop &#43; store&gt;(account: &amp;signer): EventHandle&lt;T&gt; acquires Account &#123;<br/>    event::new_event_handle(create_guid(account))<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_register_coin"></a>

## Function `register_coin`

Coin management methods.


<pre><code>public(friend) fun register_coin&lt;CoinType&gt;(account_addr: address)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun register_coin&lt;CoinType&gt;(account_addr: address) acquires Account &#123;<br/>    let account &#61; borrow_global_mut&lt;Account&gt;(account_addr);<br/>    event::emit_event&lt;CoinRegisterEvent&gt;(<br/>        &amp;mut account.coin_register_events,<br/>        CoinRegisterEvent &#123;<br/>            type_info: type_info::type_of&lt;CoinType&gt;(),<br/>        &#125;,<br/>    );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_create_signer_with_capability"></a>

## Function `create_signer_with_capability`

Capability based functions for efficient use.


<pre><code>public fun create_signer_with_capability(capability: &amp;account::SignerCapability): signer<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_signer_with_capability(capability: &amp;SignerCapability): signer &#123;<br/>    let addr &#61; &amp;capability.account;<br/>    create_signer(&#42;addr)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_get_signer_capability_address"></a>

## Function `get_signer_capability_address`



<pre><code>public fun get_signer_capability_address(capability: &amp;account::SignerCapability): address<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_signer_capability_address(capability: &amp;SignerCapability): address &#123;<br/>    capability.account<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_account_verify_signed_message"></a>

## Function `verify_signed_message`



<pre><code>public fun verify_signed_message&lt;T: drop&gt;(account: address, account_scheme: u8, account_public_key: vector&lt;u8&gt;, signed_message_bytes: vector&lt;u8&gt;, message: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun verify_signed_message&lt;T: drop&gt;(<br/>    account: address,<br/>    account_scheme: u8,<br/>    account_public_key: vector&lt;u8&gt;,<br/>    signed_message_bytes: vector&lt;u8&gt;,<br/>    message: T,<br/>) acquires Account &#123;<br/>    let account_resource &#61; borrow_global_mut&lt;Account&gt;(account);<br/>    // Verify that the `SignerCapabilityOfferProofChallengeV2` has the right information and is signed by the account owner&apos;s key<br/>    if (account_scheme &#61;&#61; ED25519_SCHEME) &#123;<br/>        let pubkey &#61; ed25519::new_unvalidated_public_key_from_bytes(account_public_key);<br/>        let expected_auth_key &#61; ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);<br/>        assert!(<br/>            account_resource.authentication_key &#61;&#61; expected_auth_key,<br/>            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY),<br/>        );<br/><br/>        let signer_capability_sig &#61; ed25519::new_signature_from_bytes(signed_message_bytes);<br/>        assert!(<br/>            ed25519::signature_verify_strict_t(&amp;signer_capability_sig, &amp;pubkey, message),<br/>            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE),<br/>        );<br/>    &#125; else if (account_scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;<br/>        let pubkey &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(account_public_key);<br/>        let expected_auth_key &#61; multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);<br/>        assert!(<br/>            account_resource.authentication_key &#61;&#61; expected_auth_key,<br/>            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY),<br/>        );<br/><br/>        let signer_capability_sig &#61; multi_ed25519::new_signature_from_bytes(signed_message_bytes);<br/>        assert!(<br/>            multi_ed25519::signature_verify_strict_t(&amp;signer_capability_sig, &amp;pubkey, message),<br/>            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE),<br/>        );<br/>    &#125; else &#123;<br/>        abort error::invalid_argument(EINVALID_SCHEME)<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

&lt;table&gt;<br/>&lt;tr&gt;<br/>&lt;th&gt;No.&lt;/th&gt;&lt;th&gt;Requirement&lt;/th&gt;&lt;th&gt;Criticality&lt;/th&gt;&lt;th&gt;Implementation&lt;/th&gt;&lt;th&gt;Enforcement&lt;/th&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;1&lt;/td&gt;<br/>&lt;td&gt;The initialization of the account module should result in the proper system initialization with valid and consistent resources.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;Initialization of the account module creates a valid address_map table and moves the resources to the OriginatingAddress under the aptos_framework account.&lt;/td&gt;<br/>&lt;td&gt;Audited that the address_map table is created and populated correctly with the expected initial values.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;2&lt;/td&gt;<br/>&lt;td&gt;After successfully creating an account, the account resources should initialize with the default data, ensuring the proper initialization of the account state.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;Creating an account via the create_account function validates the state and moves a new account resource under new_address.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;2&quot;&gt;create_account&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;3&lt;/td&gt;<br/>&lt;td&gt;Checking the existence of an account under a given address never results in an abort.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The exists_at function returns a boolean value indicating the existence of an account under the given address.&lt;/td&gt;<br/>&lt;td&gt;Formally verified by the &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;3&quot;&gt;aborts_if&lt;/a&gt; condition.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;4&lt;/td&gt;<br/>&lt;td&gt;The account module maintains bounded sequence numbers for all accounts, guaranteeing they remain within the specified limit.&lt;/td&gt;<br/>&lt;td&gt;Medium&lt;/td&gt;<br/>&lt;td&gt;The sequence number of an account may only increase up to MAX_U64 in a succeeding manner.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;4&quot;&gt;increment_sequence_number&lt;/a&gt; that it remains within the defined boundary of MAX_U64.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;5&lt;/td&gt;<br/>&lt;td&gt;Only the ed25519 and multied25519 signature schemes are permissible.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;Exclusively perform key rotation using either the ed25519 or multied25519 signature schemes. Currently restricts the offering of rotation/signing capabilities to the ed25519 or multied25519 schemes.&lt;/td&gt;<br/>&lt;td&gt;Formally Verified: &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.1&quot;&gt;rotate_authentication_key&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.2&quot;&gt;offer_rotation_capability&lt;/a&gt;, and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;5.3&quot;&gt;offer_signer_capability&lt;/a&gt;. Verified that it aborts if the account_scheme is not ED25519_SCHEME and not MULTI_ED25519_SCHEME. Audited that the scheme enums correspond correctly to signature logic.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;6&lt;/td&gt;<br/>&lt;td&gt;Exclusively permit the rotation of the authentication key of an account for the account owner or any user who possesses rotation capabilities associated with that account.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;In the rotate_authentication_key function, the authentication key derived from the from_public_key_bytes should match the signer&apos;s current authentication key. Only the delegate_signer granted the rotation capabilities may invoke the rotate_authentication_key_with_rotation_capability function.&lt;/td&gt;<br/>&lt;td&gt;Formally Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;6.1&quot;&gt;rotate_authentication_key&lt;/a&gt; and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;6.2&quot;&gt;rotate_authentication_key_with_rotation_capability&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;7&lt;/td&gt;<br/>&lt;td&gt;Only the owner of an account may offer or revoke the following capabilities: (1) offer_rotation_capability, (2) offer_signer_capability, (3) revoke_rotation_capability, and (4) revoke_signer_capability.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;An account resource may only be modified by the owner of the account utilizing: rotation_capability_offer, signer_capability_offer.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;7.1&quot;&gt;offer_rotation_capability&lt;/a&gt;, &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;7.2&quot;&gt;offer_signer_capability&lt;/a&gt;, and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;7.3&quot;&gt;revoke_rotation_capability&lt;/a&gt;. and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;7.4&quot;&gt;revoke_signer_capability&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;8&lt;/td&gt;<br/>&lt;td&gt;The capability to create a signer for the account is exclusively reserved for either the account owner or the account that has been granted the signing capabilities.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;Signer creation for the account may only be successfully executed by explicitly granting the signing capabilities with the create_authorized_signer function.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;8&quot;&gt;create_authorized_signer&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;9&lt;/td&gt;<br/>&lt;td&gt;Rotating the authentication key requires two valid signatures. With the private key of the current authentication key. With the private key of the new authentication key.&lt;/td&gt;<br/>&lt;td&gt;Critical&lt;/td&gt;<br/>&lt;td&gt;The rotate_authentication_key verifies two signatures (current and new) before rotating to the new key. The first signature ensures the user has the intended capability, and the second signature ensures that the user owns the new key.&lt;/td&gt;<br/>&lt;td&gt;Formally verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;9.1&quot;&gt;rotate_authentication_key&lt;/a&gt; and &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;9.2&quot;&gt;rotate_authentication_key_with_rotation_capability&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;10&lt;/td&gt;<br/>&lt;td&gt;The rotation of the authentication key updates the account&apos;s authentication key with the newly supplied one.&lt;/td&gt;<br/>&lt;td&gt;High&lt;/td&gt;<br/>&lt;td&gt;The auth_key may only update to the provided new_auth_key after verifying the signature.&lt;/td&gt;<br/>&lt;td&gt;Formally Verified in &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;10&quot;&gt;rotate_authentication_key_internal&lt;/a&gt; that the authentication key of an account is modified to the provided authentication key if the signature verification was successful.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;11&lt;/td&gt;<br/>&lt;td&gt;The creation number is monotonically increasing.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The guid_creation_num in the Account structure is monotonically increasing.&lt;/td&gt;<br/>&lt;td&gt;Formally Verified via &lt;a href&#61;&quot;&#35;high&#45;level&#45;req&#45;11&quot;&gt;guid_creation_num&lt;/a&gt;.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;tr&gt;<br/>&lt;td&gt;12&lt;/td&gt;<br/>&lt;td&gt;The Account resource is persistent.&lt;/td&gt;<br/>&lt;td&gt;Low&lt;/td&gt;<br/>&lt;td&gt;The Account structure assigned to the address should be persistent.&lt;/td&gt;<br/>&lt;td&gt;Audited that the Account structure is persistent.&lt;/td&gt;<br/>&lt;/tr&gt;<br/>

&lt;/table&gt;<br/>

<br/>


<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)<br/></code></pre>


Only the address <code>@aptos_framework</code> can call.<br/> OriginatingAddress does not exist under <code>@aptos_framework</code> before the call.


<pre><code>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>aborts_if exists&lt;OriginatingAddress&gt;(aptos_addr);<br/>ensures exists&lt;OriginatingAddress&gt;(aptos_addr);<br/></code></pre>



<a id="@Specification_1_create_account_if_does_not_exist"></a>

### Function `create_account_if_does_not_exist`


<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address)
>>>>>>> 13c50e058f (support mdx)
=======
<pre><code><b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
>>>>>>> 33ab75447f (back to default)
</code></pre>
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address)<br/></code></pre>
>>>>>>> fa7dac41c8 (mdx docs)


Ensure that the account exists at the end of the call.


<pre><code>let authentication_key &#61; bcs::to_bytes(account_address);<br/>aborts_if !exists&lt;Account&gt;(account_address) &amp;&amp; (<br/>    account_address &#61;&#61; @vm_reserved<br/>    &#124;&#124; account_address &#61;&#61; @aptos_framework<br/>    &#124;&#124; account_address &#61;&#61; @aptos_token<br/>    &#124;&#124; !(len(authentication_key) &#61;&#61; 32)<br/>);<br/>ensures exists&lt;Account&gt;(account_address);<br/></code></pre>



<a id="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code>public(friend) fun create_account(new_address: address): signer<br/></code></pre>


Check if the bytes of the new address is 32.<br/> The Account does not exist under the new address before creating the account.<br/> Limit the new account address is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code>include CreateAccountAbortsIf &#123;addr: new_address&#125;;<br/>aborts_if new_address &#61;&#61; @vm_reserved &#124;&#124; new_address &#61;&#61; @aptos_framework &#124;&#124; new_address &#61;&#61; @aptos_token;<br/>ensures signer::address_of(result) &#61;&#61; new_address;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 2&lt;/a&gt;:
ensures exists&lt;Account&gt;(new_address);<br/></code></pre>



<a id="@Specification_1_create_account_unchecked"></a>

### Function `create_account_unchecked`


<pre><code>fun create_account_unchecked(new_address: address): signer<br/></code></pre>


Check if the bytes of the new address is 32.<br/> The Account does not exist under the new address before creating the account.


<pre><code>include CreateAccountAbortsIf &#123;addr: new_address&#125;;<br/>ensures signer::address_of(result) &#61;&#61; new_address;<br/>ensures exists&lt;Account&gt;(new_address);<br/></code></pre>



<a id="@Specification_1_exists_at"></a>

### Function `exists_at`


<pre><code>&#35;[view]<br/>public fun exists_at(addr: address): bool<br/></code></pre>




<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt;:
aborts_if false;<br/></code></pre>




<a id="0x1_account_CreateAccountAbortsIf"></a>


<pre><code>schema CreateAccountAbortsIf &#123;<br/>addr: address;<br/>let authentication_key &#61; bcs::to_bytes(addr);<br/>aborts_if len(authentication_key) !&#61; 32;<br/>aborts_if exists&lt;Account&gt;(addr);<br/>ensures len(authentication_key) &#61;&#61; 32;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_get_guid_next_creation_num"></a>

### Function `get_guid_next_creation_num`


<pre><code>&#35;[view]<br/>public fun get_guid_next_creation_num(addr: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(addr);<br/>ensures result &#61;&#61; global&lt;Account&gt;(addr).guid_creation_num;<br/></code></pre>



<a id="@Specification_1_get_sequence_number"></a>

### Function `get_sequence_number`


<pre><code>&#35;[view]<br/>public fun get_sequence_number(addr: address): u64<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(addr);<br/>ensures result &#61;&#61; global&lt;Account&gt;(addr).sequence_number;<br/></code></pre>



<a id="@Specification_1_increment_sequence_number"></a>

### Function `increment_sequence_number`


<pre><code>public(friend) fun increment_sequence_number(addr: address)<br/></code></pre>


The Account existed under the address.<br/> The sequence_number of the Account is up to MAX_U64.


<pre><code>let sequence_number &#61; global&lt;Account&gt;(addr).sequence_number;<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 4&lt;/a&gt;:
aborts_if sequence_number &#61;&#61; MAX_U64;<br/>modifies global&lt;Account&gt;(addr);<br/>let post post_sequence_number &#61; global&lt;Account&gt;(addr).sequence_number;<br/>ensures post_sequence_number &#61;&#61; sequence_number &#43; 1;<br/></code></pre>



<a id="@Specification_1_get_authentication_key"></a>

### Function `get_authentication_key`


<pre><code>&#35;[view]<br/>public fun get_authentication_key(addr: address): vector&lt;u8&gt;<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(addr);<br/>ensures result &#61;&#61; global&lt;Account&gt;(addr).authentication_key;<br/></code></pre>



<a id="@Specification_1_rotate_authentication_key_internal"></a>

### Function `rotate_authentication_key_internal`


<pre><code>public(friend) fun rotate_authentication_key_internal(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)<br/></code></pre>


The Account existed under the signer before the call.<br/> The length of new_auth_key is 32.


<pre><code>let addr &#61; signer::address_of(account);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;10&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 10&lt;/a&gt;:
let post account_resource &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if vector::length(new_auth_key) !&#61; 32;<br/>modifies global&lt;Account&gt;(addr);<br/>ensures account_resource.authentication_key &#61;&#61; new_auth_key;<br/></code></pre>



<a id="@Specification_1_rotate_authentication_key_call"></a>

### Function `rotate_authentication_key_call`


<pre><code>entry fun rotate_authentication_key_call(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(account);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;10&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 10&lt;/a&gt;:
let post account_resource &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if vector::length(new_auth_key) !&#61; 32;<br/>modifies global&lt;Account&gt;(addr);<br/>ensures account_resource.authentication_key &#61;&#61; new_auth_key;<br/></code></pre>




<a id="0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key"></a>


<pre><code>fun spec_assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector&lt;u8&gt;, signature: vector&lt;u8&gt;, challenge: RotationProofChallenge): vector&lt;u8&gt;;<br/></code></pre>



<a id="@Specification_1_rotate_authentication_key"></a>

### Function `rotate_authentication_key`


<pre><code>public entry fun rotate_authentication_key(account: &amp;signer, from_scheme: u8, from_public_key_bytes: vector&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: vector&lt;u8&gt;, cap_rotate_key: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)<br/></code></pre>


The Account existed under the signer<br/> The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME


<pre><code>let addr &#61; signer::address_of(account);<br/>let account_resource &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;6.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 6&lt;/a&gt;:
include from_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: from_public_key_bytes &#125;;<br/>aborts_if from_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;<br/>    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);<br/>    account_resource.authentication_key !&#61; expected_auth_key<br/>&#125;);<br/>include from_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: from_public_key_bytes &#125;;<br/>aborts_if from_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;<br/>    let from_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);<br/>    account_resource.authentication_key !&#61; from_auth_key<br/>&#125;);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if from_scheme !&#61; ED25519_SCHEME &amp;&amp; from_scheme !&#61; MULTI_ED25519_SCHEME;<br/>let curr_auth_key &#61; from_bcs::deserialize&lt;address&gt;(account_resource.authentication_key);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(account_resource.authentication_key);<br/>let challenge &#61; RotationProofChallenge &#123;<br/>    sequence_number: account_resource.sequence_number,<br/>    originator: addr,<br/>    current_auth_key: curr_auth_key,<br/>    new_public_key: to_public_key_bytes,<br/>&#125;;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;9.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 9&lt;/a&gt;:
include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;<br/>    scheme: from_scheme,<br/>    public_key_bytes: from_public_key_bytes,<br/>    signature: cap_rotate_key,<br/>    challenge,<br/>&#125;;<br/>include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;<br/>    scheme: to_scheme,<br/>    public_key_bytes: to_public_key_bytes,<br/>    signature: cap_update_table,<br/>    challenge,<br/>&#125;;<br/>let originating_addr &#61; addr;<br/>let new_auth_key_vector &#61; spec_assert_valid_rotation_proof_signature_and_get_auth_key(to_scheme, to_public_key_bytes, cap_update_table, challenge);<br/>let address_map &#61; global&lt;OriginatingAddress&gt;(@aptos_framework).address_map;<br/>let new_auth_key &#61; from_bcs::deserialize&lt;address&gt;(new_auth_key_vector);<br/>aborts_if !exists&lt;OriginatingAddress&gt;(@aptos_framework);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(account_resource.authentication_key);<br/>aborts_if table::spec_contains(address_map, curr_auth_key) &amp;&amp;<br/>    table::spec_get(address_map, curr_auth_key) !&#61; originating_addr;<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(new_auth_key_vector);<br/>aborts_if curr_auth_key !&#61; new_auth_key &amp;&amp; table::spec_contains(address_map, new_auth_key);<br/>include UpdateAuthKeyAndOriginatingAddressTableAbortsIf &#123;<br/>    originating_addr: addr,<br/>&#125;;<br/>let post auth_key &#61; global&lt;Account&gt;(addr).authentication_key;<br/>ensures auth_key &#61;&#61; new_auth_key_vector;<br/></code></pre>



<a id="@Specification_1_rotate_authentication_key_with_rotation_capability"></a>

### Function `rotate_authentication_key_with_rotation_capability`


<pre><code>public entry fun rotate_authentication_key_with_rotation_capability(delegate_signer: &amp;signer, rotation_cap_offerer_address: address, new_scheme: u8, new_public_key_bytes: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(rotation_cap_offerer_address);<br/>let delegate_address &#61; signer::address_of(delegate_signer);<br/>let offerer_account_resource &#61; global&lt;Account&gt;(rotation_cap_offerer_address);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(offerer_account_resource.authentication_key);<br/>let curr_auth_key &#61; from_bcs::deserialize&lt;address&gt;(offerer_account_resource.authentication_key);<br/>aborts_if !exists&lt;Account&gt;(delegate_address);<br/>let challenge &#61; RotationProofChallenge &#123;<br/>    sequence_number: global&lt;Account&gt;(delegate_address).sequence_number,<br/>    originator: rotation_cap_offerer_address,<br/>    current_auth_key: curr_auth_key,<br/>    new_public_key: new_public_key_bytes,<br/>&#125;;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;6.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 6&lt;/a&gt;:
aborts_if !option::spec_contains(offerer_account_resource.rotation_capability_offer.for, delegate_address);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;9.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 9&lt;/a&gt;:
include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;<br/>    scheme: new_scheme,<br/>    public_key_bytes: new_public_key_bytes,<br/>    signature: cap_update_table,<br/>    challenge,<br/>&#125;;<br/>let new_auth_key_vector &#61; spec_assert_valid_rotation_proof_signature_and_get_auth_key(new_scheme, new_public_key_bytes, cap_update_table, challenge);<br/>let address_map &#61; global&lt;OriginatingAddress&gt;(@aptos_framework).address_map;<br/>aborts_if !exists&lt;OriginatingAddress&gt;(@aptos_framework);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(offerer_account_resource.authentication_key);<br/>aborts_if table::spec_contains(address_map, curr_auth_key) &amp;&amp;<br/>    table::spec_get(address_map, curr_auth_key) !&#61; rotation_cap_offerer_address;<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(new_auth_key_vector);<br/>let new_auth_key &#61; from_bcs::deserialize&lt;address&gt;(new_auth_key_vector);<br/>aborts_if curr_auth_key !&#61; new_auth_key &amp;&amp; table::spec_contains(address_map, new_auth_key);<br/>include UpdateAuthKeyAndOriginatingAddressTableAbortsIf &#123;<br/>    originating_addr: rotation_cap_offerer_address,<br/>    account_resource: offerer_account_resource,<br/>&#125;;<br/>let post auth_key &#61; global&lt;Account&gt;(rotation_cap_offerer_address).authentication_key;<br/>ensures auth_key &#61;&#61; new_auth_key_vector;<br/></code></pre>



<a id="@Specification_1_offer_rotation_capability"></a>

### Function `offer_rotation_capability`


<pre><code>public entry fun offer_rotation_capability(account: &amp;signer, rotation_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)<br/></code></pre>




<pre><code>let source_address &#61; signer::address_of(account);<br/>let account_resource &#61; global&lt;Account&gt;(source_address);<br/>let proof_challenge &#61; RotationCapabilityOfferProofChallengeV2 &#123;<br/>    chain_id: global&lt;chain_id::ChainId&gt;(@aptos_framework).id,<br/>    sequence_number: account_resource.sequence_number,<br/>    source_address,<br/>    recipient_address,<br/>&#125;;<br/>aborts_if !exists&lt;chain_id::ChainId&gt;(@aptos_framework);<br/>aborts_if !exists&lt;Account&gt;(recipient_address);<br/>aborts_if !exists&lt;Account&gt;(source_address);<br/>include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;<br/>    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);<br/>    account_resource.authentication_key !&#61; expected_auth_key<br/>&#125;);<br/>include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: rotation_capability_sig_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; !ed25519::spec_signature_verify_strict_t(<br/>    ed25519::Signature &#123; bytes: rotation_capability_sig_bytes &#125;,<br/>    ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,<br/>    proof_challenge<br/>);<br/>include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;<br/>    let expected_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);<br/>    account_resource.authentication_key !&#61; expected_auth_key<br/>&#125;);<br/>include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: rotation_capability_sig_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; !multi_ed25519::spec_signature_verify_strict_t(<br/>    multi_ed25519::Signature &#123; bytes: rotation_capability_sig_bytes &#125;,<br/>    multi_ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,<br/>    proof_challenge<br/>);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if account_scheme !&#61; ED25519_SCHEME &amp;&amp; account_scheme !&#61; MULTI_ED25519_SCHEME;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;7.1&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 7&lt;/a&gt;:
modifies global&lt;Account&gt;(source_address);<br/>let post offer_for &#61; global&lt;Account&gt;(source_address).rotation_capability_offer.for;<br/>ensures option::spec_borrow(offer_for) &#61;&#61; recipient_address;<br/></code></pre>



<a id="@Specification_1_is_rotation_capability_offered"></a>

### Function `is_rotation_capability_offered`


<pre><code>&#35;[view]<br/>public fun is_rotation_capability_offered(account_addr: address): bool<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);<br/></code></pre>



<a id="@Specification_1_get_rotation_capability_offer_for"></a>

### Function `get_rotation_capability_offer_for`


<pre><code>&#35;[view]<br/>public fun get_rotation_capability_offer_for(account_addr: address): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);<br/>let account_resource &#61; global&lt;Account&gt;(account_addr);<br/>aborts_if len(account_resource.rotation_capability_offer.for.vec) &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_revoke_rotation_capability"></a>

### Function `revoke_rotation_capability`


<pre><code>public entry fun revoke_rotation_capability(account: &amp;signer, to_be_revoked_address: address)<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(to_be_revoked_address);<br/>let addr &#61; signer::address_of(account);<br/>let account_resource &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if !option::spec_contains(account_resource.rotation_capability_offer.for,to_be_revoked_address);<br/>modifies global&lt;Account&gt;(addr);<br/>ensures exists&lt;Account&gt;(to_be_revoked_address);<br/>let post offer_for &#61; global&lt;Account&gt;(addr).rotation_capability_offer.for;<br/>ensures !option::spec_is_some(offer_for);<br/></code></pre>



<a id="@Specification_1_revoke_any_rotation_capability"></a>

### Function `revoke_any_rotation_capability`


<pre><code>public entry fun revoke_any_rotation_capability(account: &amp;signer)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(account);<br/>modifies global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>let account_resource &#61; global&lt;Account&gt;(addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;7.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 7&lt;/a&gt;:
aborts_if !option::is_some(account_resource.rotation_capability_offer.for);<br/>let post offer_for &#61; global&lt;Account&gt;(addr).rotation_capability_offer.for;<br/>ensures !option::spec_is_some(offer_for);<br/></code></pre>



<a id="@Specification_1_offer_signer_capability"></a>

### Function `offer_signer_capability`


<pre><code>public entry fun offer_signer_capability(account: &amp;signer, signer_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)<br/></code></pre>


The Account existed under the signer.<br/> The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME.


<pre><code>let source_address &#61; signer::address_of(account);<br/>let account_resource &#61; global&lt;Account&gt;(source_address);<br/>let proof_challenge &#61; SignerCapabilityOfferProofChallengeV2 &#123;<br/>    sequence_number: account_resource.sequence_number,<br/>    source_address,<br/>    recipient_address,<br/>&#125;;<br/>aborts_if !exists&lt;Account&gt;(recipient_address);<br/>aborts_if !exists&lt;Account&gt;(source_address);<br/>include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;<br/>    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);<br/>    account_resource.authentication_key !&#61; expected_auth_key<br/>&#125;);<br/>include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signer_capability_sig_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; !ed25519::spec_signature_verify_strict_t(<br/>    ed25519::Signature &#123; bytes: signer_capability_sig_bytes &#125;,<br/>    ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,<br/>    proof_challenge<br/>);<br/>include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;<br/>    let expected_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);<br/>    account_resource.authentication_key !&#61; expected_auth_key<br/>&#125;);<br/>include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signer_capability_sig_bytes &#125;;<br/>aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; !multi_ed25519::spec_signature_verify_strict_t(<br/>    multi_ed25519::Signature &#123; bytes: signer_capability_sig_bytes &#125;,<br/>    multi_ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,<br/>    proof_challenge<br/>);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;5.3&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 5&lt;/a&gt;:
aborts_if account_scheme !&#61; ED25519_SCHEME &amp;&amp; account_scheme !&#61; MULTI_ED25519_SCHEME;<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;7.2&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 7&lt;/a&gt;:
modifies global&lt;Account&gt;(source_address);<br/>let post offer_for &#61; global&lt;Account&gt;(source_address).signer_capability_offer.for;<br/>ensures option::spec_borrow(offer_for) &#61;&#61; recipient_address;<br/></code></pre>



<a id="@Specification_1_is_signer_capability_offered"></a>

### Function `is_signer_capability_offered`


<pre><code>&#35;[view]<br/>public fun is_signer_capability_offered(account_addr: address): bool<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);<br/></code></pre>



<a id="@Specification_1_get_signer_capability_offer_for"></a>

### Function `get_signer_capability_offer_for`


<pre><code>&#35;[view]<br/>public fun get_signer_capability_offer_for(account_addr: address): address<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);<br/>let account_resource &#61; global&lt;Account&gt;(account_addr);<br/>aborts_if len(account_resource.signer_capability_offer.for.vec) &#61;&#61; 0;<br/></code></pre>



<a id="@Specification_1_revoke_signer_capability"></a>

### Function `revoke_signer_capability`


<pre><code>public entry fun revoke_signer_capability(account: &amp;signer, to_be_revoked_address: address)<br/></code></pre>


The Account existed under the signer.<br/> The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address.


<pre><code>aborts_if !exists&lt;Account&gt;(to_be_revoked_address);<br/>let addr &#61; signer::address_of(account);<br/>let account_resource &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if !option::spec_contains(account_resource.signer_capability_offer.for,to_be_revoked_address);<br/>modifies global&lt;Account&gt;(addr);<br/>ensures exists&lt;Account&gt;(to_be_revoked_address);<br/></code></pre>



<a id="@Specification_1_revoke_any_signer_capability"></a>

### Function `revoke_any_signer_capability`


<pre><code>public entry fun revoke_any_signer_capability(account: &amp;signer)<br/></code></pre>




<pre><code>modifies global&lt;Account&gt;(signer::address_of(account));<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;7.4&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 7&lt;/a&gt;:
aborts_if !exists&lt;Account&gt;(signer::address_of(account));<br/>let account_resource &#61; global&lt;Account&gt;(signer::address_of(account));<br/>aborts_if !option::is_some(account_resource.signer_capability_offer.for);<br/></code></pre>



<a id="@Specification_1_create_authorized_signer"></a>

### Function `create_authorized_signer`


<pre><code>public fun create_authorized_signer(account: &amp;signer, offerer_address: address): signer<br/></code></pre>


The Account existed under the signer.<br/> The value of signer_capability_offer.for of Account resource under the signer is offerer_address.


<pre><code>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;8&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 8&lt;/a&gt;:
include AccountContainsAddr&#123;<br/>    account,<br/>    address: offerer_address,<br/>&#125;;<br/>modifies global&lt;Account&gt;(offerer_address);<br/>ensures exists&lt;Account&gt;(offerer_address);<br/>ensures signer::address_of(result) &#61;&#61; offerer_address;<br/></code></pre>




<a id="0x1_account_AccountContainsAddr"></a>


<pre><code>schema AccountContainsAddr &#123;<br/>account: signer;<br/>address: address;<br/>let addr &#61; signer::address_of(account);<br/>let account_resource &#61; global&lt;Account&gt;(address);<br/>aborts_if !exists&lt;Account&gt;(address);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;spec&#45;3&quot; href&#61;&quot;create_signer.md&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 3&lt;/a&gt; of the &lt;a href&#61;&quot;create_signer.md&quot;&gt;create_signer&lt;/a&gt; module:
    aborts_if !option::spec_contains(account_resource.signer_capability_offer.for,addr);<br/>&#125;<br/></code></pre>



<a id="@Specification_1_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

### Function `assert_valid_rotation_proof_signature_and_get_auth_key`


<pre><code>fun assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector&lt;u8&gt;, signature: vector&lt;u8&gt;, challenge: &amp;account::RotationProofChallenge): vector&lt;u8&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf;<br/>ensures [abstract] result &#61;&#61; spec_assert_valid_rotation_proof_signature_and_get_auth_key(scheme, public_key_bytes, signature, challenge);<br/></code></pre>




<a id="0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf"></a>


<pre><code>schema AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;<br/>scheme: u8;<br/>public_key_bytes: vector&lt;u8&gt;;<br/>signature: vector&lt;u8&gt;;<br/>challenge: RotationProofChallenge;<br/>include scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: public_key_bytes &#125;;<br/>include scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signature &#125;;<br/>aborts_if scheme &#61;&#61; ED25519_SCHEME &amp;&amp; !ed25519::spec_signature_verify_strict_t(<br/>    ed25519::Signature &#123; bytes: signature &#125;,<br/>    ed25519::UnvalidatedPublicKey &#123; bytes: public_key_bytes &#125;,<br/>    challenge<br/>);<br/>include scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: public_key_bytes &#125;;<br/>include scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signature &#125;;<br/>aborts_if scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; !multi_ed25519::spec_signature_verify_strict_t(<br/>    multi_ed25519::Signature &#123; bytes: signature &#125;,<br/>    multi_ed25519::UnvalidatedPublicKey &#123; bytes: public_key_bytes &#125;,<br/>    challenge<br/>);<br/>aborts_if scheme !&#61; ED25519_SCHEME &amp;&amp; scheme !&#61; MULTI_ED25519_SCHEME;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_update_auth_key_and_originating_address_table"></a>

### Function `update_auth_key_and_originating_address_table`


<pre><code>fun update_auth_key_and_originating_address_table(originating_addr: address, account_resource: &amp;mut account::Account, new_auth_key_vector: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>modifies global&lt;OriginatingAddress&gt;(@aptos_framework);<br/>include UpdateAuthKeyAndOriginatingAddressTableAbortsIf;<br/></code></pre>




<a id="0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf"></a>


<pre><code>schema UpdateAuthKeyAndOriginatingAddressTableAbortsIf &#123;<br/>originating_addr: address;<br/>account_resource: Account;<br/>new_auth_key_vector: vector&lt;u8&gt;;<br/>let address_map &#61; global&lt;OriginatingAddress&gt;(@aptos_framework).address_map;<br/>let curr_auth_key &#61; from_bcs::deserialize&lt;address&gt;(account_resource.authentication_key);<br/>let new_auth_key &#61; from_bcs::deserialize&lt;address&gt;(new_auth_key_vector);<br/>aborts_if !exists&lt;OriginatingAddress&gt;(@aptos_framework);<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(account_resource.authentication_key);<br/>aborts_if table::spec_contains(address_map, curr_auth_key) &amp;&amp;<br/>    table::spec_get(address_map, curr_auth_key) !&#61; originating_addr;<br/>aborts_if !from_bcs::deserializable&lt;address&gt;(new_auth_key_vector);<br/>aborts_if curr_auth_key !&#61; new_auth_key &amp;&amp; table::spec_contains(address_map, new_auth_key);<br/>ensures table::spec_contains(global&lt;OriginatingAddress&gt;(@aptos_framework).address_map, from_bcs::deserialize&lt;address&gt;(new_auth_key_vector));<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_resource_address"></a>

### Function `create_resource_address`


<pre><code>public fun create_resource_address(source: &amp;address, seed: vector&lt;u8&gt;): address<br/></code></pre>


The Account existed under the signer<br/> The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address


<pre><code>pragma opaque;<br/>pragma aborts_if_is_strict &#61; false;<br/>aborts_if [abstract] false;<br/>ensures [abstract] result &#61;&#61; spec_create_resource_address(source, seed);<br/></code></pre>




<a id="0x1_account_spec_create_resource_address"></a>


<pre><code>fun spec_create_resource_address(source: address, seed: vector&lt;u8&gt;): address;<br/></code></pre>



<a id="@Specification_1_create_resource_account"></a>

### Function `create_resource_account`


<pre><code>public fun create_resource_account(source: &amp;signer, seed: vector&lt;u8&gt;): (signer, account::SignerCapability)<br/></code></pre>




<pre><code>let source_addr &#61; signer::address_of(source);<br/>let resource_addr &#61; spec_create_resource_address(source_addr, seed);<br/>aborts_if len(ZERO_AUTH_KEY) !&#61; 32;<br/>include exists_at(resource_addr) &#61;&#61;&gt; CreateResourceAccountAbortsIf;<br/>include !exists_at(resource_addr) &#61;&#61;&gt; CreateAccountAbortsIf &#123;addr: resource_addr&#125;;<br/>ensures signer::address_of(result_1) &#61;&#61; resource_addr;<br/>let post offer_for &#61; global&lt;Account&gt;(resource_addr).signer_capability_offer.for;<br/>ensures option::spec_borrow(offer_for) &#61;&#61; resource_addr;<br/>ensures result_2 &#61;&#61; SignerCapability &#123; account: resource_addr &#125;;<br/></code></pre>



<a id="@Specification_1_create_framework_reserved_account"></a>

### Function `create_framework_reserved_account`


<pre><code>public(friend) fun create_framework_reserved_account(addr: address): (signer, account::SignerCapability)<br/></code></pre>


Check if the bytes of the new address is 32.<br/> The Account does not exist under the new address before creating the account.<br/> The system reserved addresses is @0x1 / @0x2 / @0x3 / @0x4 / @0x5  / @0x6 / @0x7 / @0x8 / @0x9 / @0xa.


<pre><code>aborts_if spec_is_framework_address(addr);<br/>include CreateAccountAbortsIf &#123;addr&#125;;<br/>ensures signer::address_of(result_1) &#61;&#61; addr;<br/>ensures result_2 &#61;&#61; SignerCapability &#123; account: addr &#125;;<br/></code></pre>




<a id="0x1_account_spec_is_framework_address"></a>


<pre><code>fun spec_is_framework_address(addr: address): bool&#123;<br/>   addr !&#61; @0x1 &amp;&amp;<br/>   addr !&#61; @0x2 &amp;&amp;<br/>   addr !&#61; @0x3 &amp;&amp;<br/>   addr !&#61; @0x4 &amp;&amp;<br/>   addr !&#61; @0x5 &amp;&amp;<br/>   addr !&#61; @0x6 &amp;&amp;<br/>   addr !&#61; @0x7 &amp;&amp;<br/>   addr !&#61; @0x8 &amp;&amp;<br/>   addr !&#61; @0x9 &amp;&amp;<br/>   addr !&#61; @0xa<br/>&#125;<br/></code></pre>



<a id="@Specification_1_create_guid"></a>

### Function `create_guid`


<pre><code>public fun create_guid(account_signer: &amp;signer): guid::GUID<br/></code></pre>


The Account existed under the signer.<br/> The guid_creation_num of the ccount resource is up to MAX_U64.


<pre><code>let addr &#61; signer::address_of(account_signer);<br/>include NewEventHandleAbortsIf &#123;<br/>    account: account_signer,<br/>&#125;;<br/>modifies global&lt;Account&gt;(addr);<br/>// This enforces &lt;a id&#61;&quot;high&#45;level&#45;req&#45;11&quot; href&#61;&quot;&#35;high&#45;level&#45;req&quot;&gt;high&#45;level requirement 11&lt;/a&gt;:
ensures global&lt;Account&gt;(addr).guid_creation_num &#61;&#61; old(global&lt;Account&gt;(addr).guid_creation_num) &#43; 1;<br/></code></pre>



<a id="@Specification_1_new_event_handle"></a>

### Function `new_event_handle`


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(account: &amp;signer): event::EventHandle&lt;T&gt;<br/></code></pre>


The Account existed under the signer.<br/> The guid_creation_num of the Account is up to MAX_U64.


<pre><code>include NewEventHandleAbortsIf;<br/></code></pre>




<a id="0x1_account_NewEventHandleAbortsIf"></a>


<pre><code>schema NewEventHandleAbortsIf &#123;<br/>account: &amp;signer;<br/>let addr &#61; signer::address_of(account);<br/>let account &#61; global&lt;Account&gt;(addr);<br/>aborts_if !exists&lt;Account&gt;(addr);<br/>aborts_if account.guid_creation_num &#43; 1 &gt; MAX_U64;<br/>aborts_if account.guid_creation_num &#43; 1 &gt;&#61; MAX_GUID_CREATION_NUM;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_register_coin"></a>

### Function `register_coin`


<pre><code>public(friend) fun register_coin&lt;CoinType&gt;(account_addr: address)<br/></code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);<br/>aborts_if !type_info::spec_is_struct&lt;CoinType&gt;();<br/>modifies global&lt;Account&gt;(account_addr);<br/></code></pre>



<a id="@Specification_1_create_signer_with_capability"></a>

### Function `create_signer_with_capability`


<pre><code>public fun create_signer_with_capability(capability: &amp;account::SignerCapability): signer<br/></code></pre>




<pre><code>let addr &#61; capability.account;<br/>ensures signer::address_of(result) &#61;&#61; addr;<br/></code></pre>




<a id="0x1_account_CreateResourceAccountAbortsIf"></a>


<pre><code>schema CreateResourceAccountAbortsIf &#123;<br/>resource_addr: address;<br/>let account &#61; global&lt;Account&gt;(resource_addr);<br/>aborts_if len(account.signer_capability_offer.for.vec) !&#61; 0;<br/>aborts_if account.sequence_number !&#61; 0;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_verify_signed_message"></a>

### Function `verify_signed_message`


<pre><code>public fun verify_signed_message&lt;T: drop&gt;(account: address, account_scheme: u8, account_public_key: vector&lt;u8&gt;, signed_message_bytes: vector&lt;u8&gt;, message: T)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>modifies global&lt;Account&gt;(account);<br/>let account_resource &#61; global&lt;Account&gt;(account);<br/>aborts_if !exists&lt;Account&gt;(account);<br/>include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key &#125;;<br/>aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;<br/>    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(account_public_key);<br/>    account_resource.authentication_key !&#61; expected_auth_key<br/>&#125;);<br/>include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key &#125;;<br/>aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;<br/>    let expected_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key);<br/>    account_resource.authentication_key !&#61; expected_auth_key<br/>&#125;);<br/>include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signed_message_bytes &#125;;<br/>include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signed_message_bytes &#125;;<br/>aborts_if account_scheme !&#61; ED25519_SCHEME &amp;&amp; account_scheme !&#61; MULTI_ED25519_SCHEME;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
