
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


<pre><code>use 0x1::bcs;
use 0x1::chain_id;
use 0x1::create_signer;
use 0x1::ed25519;
use 0x1::error;
use 0x1::event;
use 0x1::features;
use 0x1::from_bcs;
use 0x1::guid;
use 0x1::hash;
use 0x1::multi_ed25519;
use 0x1::option;
use 0x1::signer;
use 0x1::system_addresses;
use 0x1::table;
use 0x1::type_info;
use 0x1::vector;
</code></pre>



<a id="0x1_account_KeyRotation"></a>

## Struct `KeyRotation`



<pre><code>&#35;[event]
struct KeyRotation has drop, store
</code></pre>



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


<pre><code>struct Account has store, key
</code></pre>



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



<pre><code>struct KeyRotationEvent has drop, store
</code></pre>



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



<pre><code>struct CoinRegisterEvent has drop, store
</code></pre>



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



<pre><code>struct CapabilityOffer&lt;T&gt; has store
</code></pre>



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



<pre><code>struct RotationCapability has drop, store
</code></pre>



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



<pre><code>struct SignerCapability has drop, store
</code></pre>



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

It is easy to fetch the authentication key of an address by simply reading it from the <code>Account</code> struct at that address.
The table in this struct makes it possible to do a reverse lookup: it maps an authentication key, to the address of the account which has that authentication key set.

This mapping is needed when recovering wallets for accounts whose authentication key has been rotated.

For example, imagine a freshly-created wallet with address <code>a</code> and thus also with authentication key <code>a</code>, derived from a PK <code>pk_a</code> with corresponding SK <code>sk_a</code>.
It is easy to recover such a wallet given just the secret key <code>sk_a</code>, since the PK can be derived from the SK, the authentication key can then be derived from the PK, and the address equals the authentication key (since there was no key rotation).

However, if such a wallet rotates its authentication key to <code>b</code> derived from a different PK <code>pk_b</code> with SK <code>sk_b</code>, how would account recovery work?
The recovered address would no longer be 'a'; it would be <code>b</code>, which is incorrect.
This struct solves this problem by mapping the new authentication key <code>b</code> to the original address <code>a</code> and thus helping the wallet software during recovery find the correct address.


<pre><code>struct OriginatingAddress has key
</code></pre>



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

This structs stores the challenge message that should be signed during key rotation. First, this struct is
signed by the account owner's current public key, which proves possession of a capability to rotate the key.
Second, this struct is signed by the new public key that the account owner wants to rotate to, which proves
knowledge of this new public key's associated secret key. These two signatures cannot be replayed in another
context because they include the TXN's unique sequence number.


<pre><code>struct RotationProofChallenge has copy, drop
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

Deprecated struct - newest version is <code>RotationCapabilityOfferProofChallengeV2</code>


<pre><code>struct RotationCapabilityOfferProofChallenge has drop
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
<code>recipient_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_SignerCapabilityOfferProofChallenge"></a>

## Struct `SignerCapabilityOfferProofChallenge`

Deprecated struct - newest version is <code>SignerCapabilityOfferProofChallengeV2</code>


<pre><code>struct SignerCapabilityOfferProofChallenge has drop
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
<code>recipient_address: address</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_account_RotationCapabilityOfferProofChallengeV2"></a>

## Struct `RotationCapabilityOfferProofChallengeV2`

This struct stores the challenge message that should be signed by the source account, when the source account
is delegating its rotation capability to the <code>recipient_address</code>.
This V2 struct adds the <code>chain_id</code> and <code>source_address</code> to the challenge message, which prevents replaying the challenge message.


<pre><code>struct RotationCapabilityOfferProofChallengeV2 has drop
</code></pre>



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



<pre><code>struct SignerCapabilityOfferProofChallengeV2 has copy, drop
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



<pre><code>const MAX_U64: u128 &#61; 18446744073709551615;
</code></pre>



<a id="0x1_account_DERIVE_RESOURCE_ACCOUNT_SCHEME"></a>

Scheme identifier used when hashing an account's address together with a seed to derive the address (not the
authentication key) of a resource account. This is an abuse of the notion of a scheme identifier which, for now,
serves to domain separate hashes used to derive resource account addresses from hashes used to derive
authentication keys. Without such separation, an adversary could create (and get a signer for) a resource account
whose address matches an existing address of a MultiEd25519 wallet.


<pre><code>const DERIVE_RESOURCE_ACCOUNT_SCHEME: u8 &#61; 255;
</code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_EXISTS"></a>

Account already exists


<pre><code>const EACCOUNT_ALREADY_EXISTS: u64 &#61; 1;
</code></pre>



<a id="0x1_account_EACCOUNT_ALREADY_USED"></a>

An attempt to create a resource account on an account that has a committed transaction


<pre><code>const EACCOUNT_ALREADY_USED: u64 &#61; 16;
</code></pre>



<a id="0x1_account_EACCOUNT_DOES_NOT_EXIST"></a>

Account does not exist


<pre><code>const EACCOUNT_DOES_NOT_EXIST: u64 &#61; 2;
</code></pre>



<a id="0x1_account_ECANNOT_RESERVED_ADDRESS"></a>

Cannot create account because address is reserved


<pre><code>const ECANNOT_RESERVED_ADDRESS: u64 &#61; 5;
</code></pre>



<a id="0x1_account_ED25519_SCHEME"></a>

Scheme identifier for Ed25519 signatures used to derive authentication keys for Ed25519 public keys.


<pre><code>const ED25519_SCHEME: u8 &#61; 0;
</code></pre>



<a id="0x1_account_EEXCEEDED_MAX_GUID_CREATION_NUM"></a>



<pre><code>const EEXCEEDED_MAX_GUID_CREATION_NUM: u64 &#61; 20;
</code></pre>



<a id="0x1_account_EINVALID_ACCEPT_ROTATION_CAPABILITY"></a>

The caller does not have a valid rotation capability offer from the other account


<pre><code>const EINVALID_ACCEPT_ROTATION_CAPABILITY: u64 &#61; 10;
</code></pre>



<a id="0x1_account_EINVALID_ORIGINATING_ADDRESS"></a>

Abort the transaction if the expected originating address is different from the originating address on-chain


<pre><code>const EINVALID_ORIGINATING_ADDRESS: u64 &#61; 13;
</code></pre>



<a id="0x1_account_EINVALID_PROOF_OF_KNOWLEDGE"></a>

Specified proof of knowledge required to prove ownership of a public key is invalid


<pre><code>const EINVALID_PROOF_OF_KNOWLEDGE: u64 &#61; 8;
</code></pre>



<a id="0x1_account_EINVALID_SCHEME"></a>

Specified scheme required to proceed with the smart contract operation - can only be ED25519_SCHEME(0) OR MULTI_ED25519_SCHEME(1)


<pre><code>const EINVALID_SCHEME: u64 &#61; 12;
</code></pre>



<a id="0x1_account_EMALFORMED_AUTHENTICATION_KEY"></a>

The provided authentication key has an invalid length


<pre><code>const EMALFORMED_AUTHENTICATION_KEY: u64 &#61; 4;
</code></pre>



<a id="0x1_account_ENO_CAPABILITY"></a>

The caller does not have a digital-signature-based capability to call this function


<pre><code>const ENO_CAPABILITY: u64 &#61; 9;
</code></pre>



<a id="0x1_account_ENO_SIGNER_CAPABILITY_OFFERED"></a>



<pre><code>const ENO_SIGNER_CAPABILITY_OFFERED: u64 &#61; 19;
</code></pre>



<a id="0x1_account_ENO_SUCH_ROTATION_CAPABILITY_OFFER"></a>

The specified rotation capablity offer does not exist at the specified offerer address


<pre><code>const ENO_SUCH_ROTATION_CAPABILITY_OFFER: u64 &#61; 18;
</code></pre>



<a id="0x1_account_ENO_SUCH_SIGNER_CAPABILITY"></a>

The signer capability offer doesn't exist at the given address


<pre><code>const ENO_SUCH_SIGNER_CAPABILITY: u64 &#61; 14;
</code></pre>



<a id="0x1_account_ENO_VALID_FRAMEWORK_RESERVED_ADDRESS"></a>

Address to create is not a valid reserved address for Aptos framework


<pre><code>const ENO_VALID_FRAMEWORK_RESERVED_ADDRESS: u64 &#61; 11;
</code></pre>



<a id="0x1_account_EOFFERER_ADDRESS_DOES_NOT_EXIST"></a>

Offerer address doesn't exist


<pre><code>const EOFFERER_ADDRESS_DOES_NOT_EXIST: u64 &#61; 17;
</code></pre>



<a id="0x1_account_EOUT_OF_GAS"></a>

Transaction exceeded its allocated max gas


<pre><code>const EOUT_OF_GAS: u64 &#61; 6;
</code></pre>



<a id="0x1_account_ERESOURCE_ACCCOUNT_EXISTS"></a>

An attempt to create a resource account on a claimed account


<pre><code>const ERESOURCE_ACCCOUNT_EXISTS: u64 &#61; 15;
</code></pre>



<a id="0x1_account_ESEQUENCE_NUMBER_TOO_BIG"></a>

Sequence number exceeds the maximum value for a u64


<pre><code>const ESEQUENCE_NUMBER_TOO_BIG: u64 &#61; 3;
</code></pre>



<a id="0x1_account_EWRONG_CURRENT_PUBLIC_KEY"></a>

Specified current public key is not correct


<pre><code>const EWRONG_CURRENT_PUBLIC_KEY: u64 &#61; 7;
</code></pre>



<a id="0x1_account_MAX_GUID_CREATION_NUM"></a>

Explicitly separate the GUID space between Object and Account to prevent accidental overlap.


<pre><code>const MAX_GUID_CREATION_NUM: u64 &#61; 1125899906842624;
</code></pre>



<a id="0x1_account_MULTI_ED25519_SCHEME"></a>

Scheme identifier for MultiEd25519 signatures used to derive authentication keys for MultiEd25519 public keys.


<pre><code>const MULTI_ED25519_SCHEME: u8 &#61; 1;
</code></pre>



<a id="0x1_account_ZERO_AUTH_KEY"></a>



<pre><code>const ZERO_AUTH_KEY: vector&lt;u8&gt; &#61; [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
</code></pre>



<a id="0x1_account_initialize"></a>

## Function `initialize`

Only called during genesis to initialize system resources for this module.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    move_to(aptos_framework, OriginatingAddress &#123;
        address_map: table::new(),
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_account_create_account_if_does_not_exist"></a>

## Function `create_account_if_does_not_exist`



<<<<<<< HEAD
<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address)
>>>>>>> 13c50e058f (support mdx)
</code></pre>



<details>
<summary>Implementation</summary>


<<<<<<< HEAD
<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>) {
    <b>if</b> (!<b>exists</b>&lt;<a href="account.md#0x1_account_Account">Account</a>&gt;(account_address)) {
        <a href="account.md#0x1_account_create_account">create_account</a>(account_address);
    }
}
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address) &#123;
    if (!exists&lt;Account&gt;(account_address)) &#123;
        create_account(account_address);
    &#125;
&#125;
>>>>>>> 13c50e058f (support mdx)
</code></pre>



</details>

<a id="0x1_account_create_account"></a>

## Function `create_account`

Publishes a new <code>Account</code> resource under <code>new_address</code>. A signer representing <code>new_address</code>
is returned. This way, the caller of this function can publish additional resources under
<code>new_address</code>.


<pre><code>public(friend) fun create_account(new_address: address): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_account(new_address: address): signer &#123;
    // there cannot be an Account resource under new_addr already.
    assert!(!exists&lt;Account&gt;(new_address), error::already_exists(EACCOUNT_ALREADY_EXISTS));

    // NOTE: @core_resources gets created via a `create_account` call, so we do not include it below.
    assert!(
        new_address !&#61; @vm_reserved &amp;&amp; new_address !&#61; @aptos_framework &amp;&amp; new_address !&#61; @aptos_token,
        error::invalid_argument(ECANNOT_RESERVED_ADDRESS)
    );

    create_account_unchecked(new_address)
&#125;
</code></pre>



</details>

<a id="0x1_account_create_account_unchecked"></a>

## Function `create_account_unchecked`



<pre><code>fun create_account_unchecked(new_address: address): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun create_account_unchecked(new_address: address): signer &#123;
    let new_account &#61; create_signer(new_address);
    let authentication_key &#61; bcs::to_bytes(&amp;new_address);
    assert!(
        vector::length(&amp;authentication_key) &#61;&#61; 32,
        error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
    );

    let guid_creation_num &#61; 0;

    let guid_for_coin &#61; guid::create(new_address, &amp;mut guid_creation_num);
    let coin_register_events &#61; event::new_event_handle&lt;CoinRegisterEvent&gt;(guid_for_coin);

    let guid_for_rotation &#61; guid::create(new_address, &amp;mut guid_creation_num);
    let key_rotation_events &#61; event::new_event_handle&lt;KeyRotationEvent&gt;(guid_for_rotation);

    move_to(
        &amp;new_account,
        Account &#123;
            authentication_key,
            sequence_number: 0,
            guid_creation_num,
            coin_register_events,
            key_rotation_events,
            rotation_capability_offer: CapabilityOffer &#123; for: option::none() &#125;,
            signer_capability_offer: CapabilityOffer &#123; for: option::none() &#125;,
        &#125;
    );

    new_account
&#125;
</code></pre>



</details>

<a id="0x1_account_exists_at"></a>

## Function `exists_at`



<pre><code>&#35;[view]
public fun exists_at(addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun exists_at(addr: address): bool &#123;
    exists&lt;Account&gt;(addr)
&#125;
</code></pre>



</details>

<a id="0x1_account_get_guid_next_creation_num"></a>

## Function `get_guid_next_creation_num`



<pre><code>&#35;[view]
public fun get_guid_next_creation_num(addr: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_guid_next_creation_num(addr: address): u64 acquires Account &#123;
    borrow_global&lt;Account&gt;(addr).guid_creation_num
&#125;
</code></pre>



</details>

<a id="0x1_account_get_sequence_number"></a>

## Function `get_sequence_number`



<pre><code>&#35;[view]
public fun get_sequence_number(addr: address): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_sequence_number(addr: address): u64 acquires Account &#123;
    borrow_global&lt;Account&gt;(addr).sequence_number
&#125;
</code></pre>



</details>

<a id="0x1_account_increment_sequence_number"></a>

## Function `increment_sequence_number`



<pre><code>public(friend) fun increment_sequence_number(addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun increment_sequence_number(addr: address) acquires Account &#123;
    let sequence_number &#61; &amp;mut borrow_global_mut&lt;Account&gt;(addr).sequence_number;

    assert!(
        (&#42;sequence_number as u128) &lt; MAX_U64,
        error::out_of_range(ESEQUENCE_NUMBER_TOO_BIG)
    );

    &#42;sequence_number &#61; &#42;sequence_number &#43; 1;
&#125;
</code></pre>



</details>

<a id="0x1_account_get_authentication_key"></a>

## Function `get_authentication_key`



<pre><code>&#35;[view]
public fun get_authentication_key(addr: address): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_authentication_key(addr: address): vector&lt;u8&gt; acquires Account &#123;
    borrow_global&lt;Account&gt;(addr).authentication_key
&#125;
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_internal"></a>

## Function `rotate_authentication_key_internal`

This function is used to rotate a resource account's authentication key to <code>new_auth_key</code>. This is done in
many contexts:
1. During normal key rotation via <code>rotate_authentication_key</code> or <code>rotate_authentication_key_call</code>
2. During resource account initialization so that no private key can control the resource account
3. During multisig_v2 account creation


<pre><code>public(friend) fun rotate_authentication_key_internal(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun rotate_authentication_key_internal(account: &amp;signer, new_auth_key: vector&lt;u8&gt;) acquires Account &#123;
    let addr &#61; signer::address_of(account);
    assert!(exists_at(addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));
    assert!(
        vector::length(&amp;new_auth_key) &#61;&#61; 32,
        error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
    );
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);
    account_resource.authentication_key &#61; new_auth_key;
&#125;
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_call"></a>

## Function `rotate_authentication_key_call`

Private entry function for key rotation that allows the signer to update their authentication key.
Note that this does not update the <code>OriginatingAddress</code> table because the <code>new_auth_key</code> is not "verified": it
does not come with a proof-of-knowledge of the underlying SK. Nonetheless, we need this functionality due to
the introduction of non-standard key algorithms, such as passkeys, which cannot produce proofs-of-knowledge in
the format expected in <code>rotate_authentication_key</code>.


<pre><code>entry fun rotate_authentication_key_call(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>entry fun rotate_authentication_key_call(account: &amp;signer, new_auth_key: vector&lt;u8&gt;) acquires Account &#123;
    rotate_authentication_key_internal(account, new_auth_key);
&#125;
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key"></a>

## Function `rotate_authentication_key`

Generic authentication key rotation function that allows the user to rotate their authentication key from any scheme to any scheme.
To authorize the rotation, we need two signatures:
- the first signature <code>cap_rotate_key</code> refers to the signature by the account owner's current key on a valid <code>RotationProofChallenge</code>,
demonstrating that the user intends to and has the capability to rotate the authentication key of this account;
- the second signature <code>cap_update_table</code> refers to the signature by the new key (that the account owner wants to rotate to) on a
valid <code>RotationProofChallenge</code>, demonstrating that the user owns the new private key, and has the authority to update the
<code>OriginatingAddress</code> map with the new address mapping <code>&lt;new_address, originating_address&gt;</code>.
To verify these two signatures, we need their corresponding public key and public key scheme: we use <code>from_scheme</code> and <code>from_public_key_bytes</code>
to verify <code>cap_rotate_key</code>, and <code>to_scheme</code> and <code>to_public_key_bytes</code> to verify <code>cap_update_table</code>.
A scheme of 0 refers to an Ed25519 key and a scheme of 1 refers to Multi-Ed25519 keys.
<code>originating address</code> refers to an account's original/first address.

Here is an example attack if we don't ask for the second signature <code>cap_update_table</code>:
Alice has rotated her account <code>addr_a</code> to <code>new_addr_a</code>. As a result, the following entry is created, to help Alice when recovering her wallet:
<code>OriginatingAddress[new_addr_a]</code> -> <code>addr_a</code>
Alice has had bad day: her laptop blew up and she needs to reset her account on a new one.
(Fortunately, she still has her secret key <code>new_sk_a</code> associated with her new address <code>new_addr_a</code>, so she can do this.)

But Bob likes to mess with Alice.
Bob creates an account <code>addr_b</code> and maliciously rotates it to Alice's new address <code>new_addr_a</code>. Since we are no longer checking a PoK,
Bob can easily do this.

Now, the table will be updated to make Alice's new address point to Bob's address: <code>OriginatingAddress[new_addr_a]</code> -> <code>addr_b</code>.
When Alice recovers her account, her wallet will display the attacker's address (Bob's) <code>addr_b</code> as her address.
Now Alice will give <code>addr_b</code> to everyone to pay her, but the money will go to Bob.

Because we ask for a valid <code>cap_update_table</code>, this kind of attack is not possible. Bob would not have the secret key of Alice's address
to rotate his address to Alice's address in the first place.


<pre><code>public entry fun rotate_authentication_key(account: &amp;signer, from_scheme: u8, from_public_key_bytes: vector&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: vector&lt;u8&gt;, cap_rotate_key: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun rotate_authentication_key(
    account: &amp;signer,
    from_scheme: u8,
    from_public_key_bytes: vector&lt;u8&gt;,
    to_scheme: u8,
    to_public_key_bytes: vector&lt;u8&gt;,
    cap_rotate_key: vector&lt;u8&gt;,
    cap_update_table: vector&lt;u8&gt;,
) acquires Account, OriginatingAddress &#123;
    let addr &#61; signer::address_of(account);
    assert!(exists_at(addr), error::not_found(EACCOUNT_DOES_NOT_EXIST));
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);

    // Verify the given `from_public_key_bytes` matches this account&apos;s current authentication key.
    if (from_scheme &#61;&#61; ED25519_SCHEME) &#123;
        let from_pk &#61; ed25519::new_unvalidated_public_key_from_bytes(from_public_key_bytes);
        let from_auth_key &#61; ed25519::unvalidated_public_key_to_authentication_key(&amp;from_pk);
        assert!(
            account_resource.authentication_key &#61;&#61; from_auth_key,
            error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY)
        );
    &#125; else if (from_scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;
        let from_pk &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(from_public_key_bytes);
        let from_auth_key &#61; multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;from_pk);
        assert!(
            account_resource.authentication_key &#61;&#61; from_auth_key,
            error::unauthenticated(EWRONG_CURRENT_PUBLIC_KEY)
        );
    &#125; else &#123;
        abort error::invalid_argument(EINVALID_SCHEME)
    &#125;;

    // Construct a valid `RotationProofChallenge` that `cap_rotate_key` and `cap_update_table` will validate against.
    let curr_auth_key_as_address &#61; from_bcs::to_address(account_resource.authentication_key);
    let challenge &#61; RotationProofChallenge &#123;
        sequence_number: account_resource.sequence_number,
        originator: addr,
        current_auth_key: curr_auth_key_as_address,
        new_public_key: to_public_key_bytes,
    &#125;;

    // Assert the challenges signed by the current and new keys are valid
    assert_valid_rotation_proof_signature_and_get_auth_key(
        from_scheme,
        from_public_key_bytes,
        cap_rotate_key,
        &amp;challenge
    );
    let new_auth_key &#61; assert_valid_rotation_proof_signature_and_get_auth_key(
        to_scheme,
        to_public_key_bytes,
        cap_update_table,
        &amp;challenge
    );

    // Update the `OriginatingAddress` table.
    update_auth_key_and_originating_address_table(addr, account_resource, new_auth_key);
&#125;
</code></pre>



</details>

<a id="0x1_account_rotate_authentication_key_with_rotation_capability"></a>

## Function `rotate_authentication_key_with_rotation_capability`



<pre><code>public entry fun rotate_authentication_key_with_rotation_capability(delegate_signer: &amp;signer, rotation_cap_offerer_address: address, new_scheme: u8, new_public_key_bytes: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun rotate_authentication_key_with_rotation_capability(
    delegate_signer: &amp;signer,
    rotation_cap_offerer_address: address,
    new_scheme: u8,
    new_public_key_bytes: vector&lt;u8&gt;,
    cap_update_table: vector&lt;u8&gt;
) acquires Account, OriginatingAddress &#123;
    assert!(exists_at(rotation_cap_offerer_address), error::not_found(EOFFERER_ADDRESS_DOES_NOT_EXIST));

    // Check that there exists a rotation capability offer at the offerer&apos;s account resource for the delegate.
    let delegate_address &#61; signer::address_of(delegate_signer);
    let offerer_account_resource &#61; borrow_global&lt;Account&gt;(rotation_cap_offerer_address);
    assert!(
        option::contains(&amp;offerer_account_resource.rotation_capability_offer.for, &amp;delegate_address),
        error::not_found(ENO_SUCH_ROTATION_CAPABILITY_OFFER)
    );

    let curr_auth_key &#61; from_bcs::to_address(offerer_account_resource.authentication_key);
    let challenge &#61; RotationProofChallenge &#123;
        sequence_number: get_sequence_number(delegate_address),
        originator: rotation_cap_offerer_address,
        current_auth_key: curr_auth_key,
        new_public_key: new_public_key_bytes,
    &#125;;

    // Verifies that the `RotationProofChallenge` from above is signed under the new public key that we are rotating to.        l
    let new_auth_key &#61; assert_valid_rotation_proof_signature_and_get_auth_key(
        new_scheme,
        new_public_key_bytes,
        cap_update_table,
        &amp;challenge
    );

    // Update the `OriginatingAddress` table, so we can find the originating address using the new address.
    let offerer_account_resource &#61; borrow_global_mut&lt;Account&gt;(rotation_cap_offerer_address);
    update_auth_key_and_originating_address_table(
        rotation_cap_offerer_address,
        offerer_account_resource,
        new_auth_key
    );
&#125;
</code></pre>



</details>

<a id="0x1_account_offer_rotation_capability"></a>

## Function `offer_rotation_capability`

Offers rotation capability on behalf of <code>account</code> to the account at address <code>recipient_address</code>.
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

@param rotation_capability_sig_bytes is the signature by the account owner's key on <code>RotationCapabilityOfferProofChallengeV2</code>.
@param account_scheme is the scheme of the account (ed25519 or multi_ed25519).
@param account_public_key_bytes is the public key of the account owner.
@param recipient_address is the address of the recipient of the rotation capability - note that if there's an existing rotation capability
offer, calling this function will replace the previous <code>recipient_address</code> upon successful verification.


<pre><code>public entry fun offer_rotation_capability(account: &amp;signer, rotation_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun offer_rotation_capability(
    account: &amp;signer,
    rotation_capability_sig_bytes: vector&lt;u8&gt;,
    account_scheme: u8,
    account_public_key_bytes: vector&lt;u8&gt;,
    recipient_address: address,
) acquires Account &#123;
    let addr &#61; signer::address_of(account);
    assert!(exists_at(recipient_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

    // proof that this account intends to delegate its rotation capability to another account
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);
    let proof_challenge &#61; RotationCapabilityOfferProofChallengeV2 &#123;
        chain_id: chain_id::get(),
        sequence_number: account_resource.sequence_number,
        source_address: addr,
        recipient_address,
    &#125;;

    // verify the signature on `RotationCapabilityOfferProofChallengeV2` by the account owner
    if (account_scheme &#61;&#61; ED25519_SCHEME) &#123;
        let pubkey &#61; ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);
        let expected_auth_key &#61; ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);
        assert!(
            account_resource.authentication_key &#61;&#61; expected_auth_key,
            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY)
        );

        let rotation_capability_sig &#61; ed25519::new_signature_from_bytes(rotation_capability_sig_bytes);
        assert!(
            ed25519::signature_verify_strict_t(&amp;rotation_capability_sig, &amp;pubkey, proof_challenge),
            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)
        );
    &#125; else if (account_scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;
        let pubkey &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(account_public_key_bytes);
        let expected_auth_key &#61; multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);
        assert!(
            account_resource.authentication_key &#61;&#61; expected_auth_key,
            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY)
        );

        let rotation_capability_sig &#61; multi_ed25519::new_signature_from_bytes(rotation_capability_sig_bytes);
        assert!(
            multi_ed25519::signature_verify_strict_t(&amp;rotation_capability_sig, &amp;pubkey, proof_challenge),
            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)
        );
    &#125; else &#123;
        abort error::invalid_argument(EINVALID_SCHEME)
    &#125;;

    // update the existing rotation capability offer or put in a new rotation capability offer for the current account
    option::swap_or_fill(&amp;mut account_resource.rotation_capability_offer.for, recipient_address);
&#125;
</code></pre>



</details>

<a id="0x1_account_is_rotation_capability_offered"></a>

## Function `is_rotation_capability_offered`

Returns true if the account at <code>account_addr</code> has a rotation capability offer.


<pre><code>&#35;[view]
public fun is_rotation_capability_offered(account_addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_rotation_capability_offered(account_addr: address): bool acquires Account &#123;
    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);
    option::is_some(&amp;account_resource.rotation_capability_offer.for)
&#125;
</code></pre>



</details>

<a id="0x1_account_get_rotation_capability_offer_for"></a>

## Function `get_rotation_capability_offer_for`

Returns the address of the account that has a rotation capability offer from the account at <code>account_addr</code>.


<pre><code>&#35;[view]
public fun get_rotation_capability_offer_for(account_addr: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_rotation_capability_offer_for(account_addr: address): address acquires Account &#123;
    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);
    assert!(
        option::is_some(&amp;account_resource.rotation_capability_offer.for),
        error::not_found(ENO_SIGNER_CAPABILITY_OFFERED),
    );
    &#42;option::borrow(&amp;account_resource.rotation_capability_offer.for)
&#125;
</code></pre>



</details>

<a id="0x1_account_revoke_rotation_capability"></a>

## Function `revoke_rotation_capability`

Revoke the rotation capability offer given to <code>to_be_revoked_recipient_address</code> from <code>account</code>


<pre><code>public entry fun revoke_rotation_capability(account: &amp;signer, to_be_revoked_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_rotation_capability(account: &amp;signer, to_be_revoked_address: address) acquires Account &#123;
    assert!(exists_at(to_be_revoked_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));
    let addr &#61; signer::address_of(account);
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);
    assert!(
        option::contains(&amp;account_resource.rotation_capability_offer.for, &amp;to_be_revoked_address),
        error::not_found(ENO_SUCH_ROTATION_CAPABILITY_OFFER)
    );
    revoke_any_rotation_capability(account);
&#125;
</code></pre>



</details>

<a id="0x1_account_revoke_any_rotation_capability"></a>

## Function `revoke_any_rotation_capability`

Revoke any rotation capability offer in the specified account.


<pre><code>public entry fun revoke_any_rotation_capability(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_any_rotation_capability(account: &amp;signer) acquires Account &#123;
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(signer::address_of(account));
    option::extract(&amp;mut account_resource.rotation_capability_offer.for);
&#125;
</code></pre>



</details>

<a id="0x1_account_offer_signer_capability"></a>

## Function `offer_signer_capability`

Offers signer capability on behalf of <code>account</code> to the account at address <code>recipient_address</code>.
An account can delegate its signer capability to only one other address at one time.
<code>signer_capability_key_bytes</code> is the <code>SignerCapabilityOfferProofChallengeV2</code> signed by the account owner's key
<code>account_scheme</code> is the scheme of the account (ed25519 or multi_ed25519).
<code>account_public_key_bytes</code> is the public key of the account owner.
<code>recipient_address</code> is the address of the recipient of the signer capability - note that if there's an existing
<code>recipient_address</code> in the account owner's <code>SignerCapabilityOffer</code>, this will replace the
previous <code>recipient_address</code> upon successful verification (the previous recipient will no longer have access
to the account owner's signer capability).


<pre><code>public entry fun offer_signer_capability(account: &amp;signer, signer_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun offer_signer_capability(
    account: &amp;signer,
    signer_capability_sig_bytes: vector&lt;u8&gt;,
    account_scheme: u8,
    account_public_key_bytes: vector&lt;u8&gt;,
    recipient_address: address
) acquires Account &#123;
    let source_address &#61; signer::address_of(account);
    assert!(exists_at(recipient_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));

    // Proof that this account intends to delegate its signer capability to another account.
    let proof_challenge &#61; SignerCapabilityOfferProofChallengeV2 &#123;
        sequence_number: get_sequence_number(source_address),
        source_address,
        recipient_address,
    &#125;;
    verify_signed_message(
        source_address, account_scheme, account_public_key_bytes, signer_capability_sig_bytes, proof_challenge);

    // Update the existing signer capability offer or put in a new signer capability offer for the recipient.
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(source_address);
    option::swap_or_fill(&amp;mut account_resource.signer_capability_offer.for, recipient_address);
&#125;
</code></pre>



</details>

<a id="0x1_account_is_signer_capability_offered"></a>

## Function `is_signer_capability_offered`

Returns true if the account at <code>account_addr</code> has a signer capability offer.


<pre><code>&#35;[view]
public fun is_signer_capability_offered(account_addr: address): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_signer_capability_offered(account_addr: address): bool acquires Account &#123;
    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);
    option::is_some(&amp;account_resource.signer_capability_offer.for)
&#125;
</code></pre>



</details>

<a id="0x1_account_get_signer_capability_offer_for"></a>

## Function `get_signer_capability_offer_for`

Returns the address of the account that has a signer capability offer from the account at <code>account_addr</code>.


<pre><code>&#35;[view]
public fun get_signer_capability_offer_for(account_addr: address): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_signer_capability_offer_for(account_addr: address): address acquires Account &#123;
    let account_resource &#61; borrow_global&lt;Account&gt;(account_addr);
    assert!(
        option::is_some(&amp;account_resource.signer_capability_offer.for),
        error::not_found(ENO_SIGNER_CAPABILITY_OFFERED),
    );
    &#42;option::borrow(&amp;account_resource.signer_capability_offer.for)
&#125;
</code></pre>



</details>

<a id="0x1_account_revoke_signer_capability"></a>

## Function `revoke_signer_capability`

Revoke the account owner's signer capability offer for <code>to_be_revoked_address</code> (i.e., the address that
has a signer capability offer from <code>account</code> but will be revoked in this function).


<pre><code>public entry fun revoke_signer_capability(account: &amp;signer, to_be_revoked_address: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_signer_capability(account: &amp;signer, to_be_revoked_address: address) acquires Account &#123;
    assert!(exists_at(to_be_revoked_address), error::not_found(EACCOUNT_DOES_NOT_EXIST));
    let addr &#61; signer::address_of(account);
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(addr);
    assert!(
        option::contains(&amp;account_resource.signer_capability_offer.for, &amp;to_be_revoked_address),
        error::not_found(ENO_SUCH_SIGNER_CAPABILITY)
    );
    revoke_any_signer_capability(account);
&#125;
</code></pre>



</details>

<a id="0x1_account_revoke_any_signer_capability"></a>

## Function `revoke_any_signer_capability`

Revoke any signer capability offer in the specified account.


<pre><code>public entry fun revoke_any_signer_capability(account: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun revoke_any_signer_capability(account: &amp;signer) acquires Account &#123;
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(signer::address_of(account));
    option::extract(&amp;mut account_resource.signer_capability_offer.for);
&#125;
</code></pre>



</details>

<a id="0x1_account_create_authorized_signer"></a>

## Function `create_authorized_signer`

Return an authorized signer of the offerer, if there's an existing signer capability offer for <code>account</code>
at the offerer's address.


<pre><code>public fun create_authorized_signer(account: &amp;signer, offerer_address: address): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_authorized_signer(account: &amp;signer, offerer_address: address): signer acquires Account &#123;
    assert!(exists_at(offerer_address), error::not_found(EOFFERER_ADDRESS_DOES_NOT_EXIST));

    // Check if there&apos;s an existing signer capability offer from the offerer.
    let account_resource &#61; borrow_global&lt;Account&gt;(offerer_address);
    let addr &#61; signer::address_of(account);
    assert!(
        option::contains(&amp;account_resource.signer_capability_offer.for, &amp;addr),
        error::not_found(ENO_SUCH_SIGNER_CAPABILITY)
    );

    create_signer(offerer_address)
&#125;
</code></pre>



</details>

<a id="0x1_account_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

## Function `assert_valid_rotation_proof_signature_and_get_auth_key`

Helper functions for authentication key rotation.


<pre><code>fun assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector&lt;u8&gt;, signature: vector&lt;u8&gt;, challenge: &amp;account::RotationProofChallenge): vector&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun assert_valid_rotation_proof_signature_and_get_auth_key(
    scheme: u8,
    public_key_bytes: vector&lt;u8&gt;,
    signature: vector&lt;u8&gt;,
    challenge: &amp;RotationProofChallenge
): vector&lt;u8&gt; &#123;
    if (scheme &#61;&#61; ED25519_SCHEME) &#123;
        let pk &#61; ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
        let sig &#61; ed25519::new_signature_from_bytes(signature);
        assert!(
            ed25519::signature_verify_strict_t(&amp;sig, &amp;pk, &#42;challenge),
            std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)
        );
        ed25519::unvalidated_public_key_to_authentication_key(&amp;pk)
    &#125; else if (scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;
        let pk &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(public_key_bytes);
        let sig &#61; multi_ed25519::new_signature_from_bytes(signature);
        assert!(
            multi_ed25519::signature_verify_strict_t(&amp;sig, &amp;pk, &#42;challenge),
            std::error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE)
        );
        multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;pk)
    &#125; else &#123;
        abort error::invalid_argument(EINVALID_SCHEME)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_account_update_auth_key_and_originating_address_table"></a>

## Function `update_auth_key_and_originating_address_table`

Update the <code>OriginatingAddress</code> table, so that we can find the originating address using the latest address
in the event of key recovery.


<pre><code>fun update_auth_key_and_originating_address_table(originating_addr: address, account_resource: &amp;mut account::Account, new_auth_key_vector: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun update_auth_key_and_originating_address_table(
    originating_addr: address,
    account_resource: &amp;mut Account,
    new_auth_key_vector: vector&lt;u8&gt;,
) acquires OriginatingAddress &#123;
    let address_map &#61; &amp;mut borrow_global_mut&lt;OriginatingAddress&gt;(@aptos_framework).address_map;
    let curr_auth_key &#61; from_bcs::to_address(account_resource.authentication_key);

    // Checks `OriginatingAddress[curr_auth_key]` is either unmapped, or mapped to `originating_address`.
    // If it&apos;s mapped to the originating address, removes that mapping.
    // Otherwise, abort if it&apos;s mapped to a different address.
    if (table::contains(address_map, curr_auth_key)) &#123;
        // If account_a with address_a is rotating its keypair from keypair_a to keypair_b, we expect
        // the address of the account to stay the same, while its keypair updates to keypair_b.
        // Here, by asserting that we&apos;re calling from the account with the originating address, we enforce
        // the standard of keeping the same address and updating the keypair at the contract level.
        // Without this assertion, the dapps could also update the account&apos;s address to address_b (the address that
        // is programmatically related to keypaier_b) and update the keypair to keypair_b. This causes problems
        // for interoperability because different dapps can implement this in different ways.
        // If the account with address b calls this function with two valid signatures, it will abort at this step,
        // because address b is not the account&apos;s originating address.
        assert!(
            originating_addr &#61;&#61; table::remove(address_map, curr_auth_key),
            error::not_found(EINVALID_ORIGINATING_ADDRESS)
        );
    &#125;;

    // Set `OriginatingAddress[new_auth_key] &#61; originating_address`.
    let new_auth_key &#61; from_bcs::to_address(new_auth_key_vector);
    table::add(address_map, new_auth_key, originating_addr);

    if (std::features::module_event_migration_enabled()) &#123;
        event::emit(KeyRotation &#123;
            account: originating_addr,
            old_authentication_key: account_resource.authentication_key,
            new_authentication_key: new_auth_key_vector,
        &#125;);
    &#125;;
    event::emit_event&lt;KeyRotationEvent&gt;(
        &amp;mut account_resource.key_rotation_events,
        KeyRotationEvent &#123;
            old_authentication_key: account_resource.authentication_key,
            new_authentication_key: new_auth_key_vector,
        &#125;
    );

    // Update the account resource&apos;s authentication key.
    account_resource.authentication_key &#61; new_auth_key_vector;
&#125;
</code></pre>



</details>

<a id="0x1_account_create_resource_address"></a>

## Function `create_resource_address`

Basic account creation methods.
This is a helper function to compute resource addresses. Computation of the address
involves the use of a cryptographic hash operation and should be use thoughtfully.


<pre><code>public fun create_resource_address(source: &amp;address, seed: vector&lt;u8&gt;): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_resource_address(source: &amp;address, seed: vector&lt;u8&gt;): address &#123;
    let bytes &#61; bcs::to_bytes(source);
    vector::append(&amp;mut bytes, seed);
    vector::push_back(&amp;mut bytes, DERIVE_RESOURCE_ACCOUNT_SCHEME);
    from_bcs::to_address(hash::sha3_256(bytes))
&#125;
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


<pre><code>public fun create_resource_account(source: &amp;signer, seed: vector&lt;u8&gt;): (signer, account::SignerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_resource_account(source: &amp;signer, seed: vector&lt;u8&gt;): (signer, SignerCapability) acquires Account &#123;
    let resource_addr &#61; create_resource_address(&amp;signer::address_of(source), seed);
    let resource &#61; if (exists_at(resource_addr)) &#123;
        let account &#61; borrow_global&lt;Account&gt;(resource_addr);
        assert!(
            option::is_none(&amp;account.signer_capability_offer.for),
            error::already_exists(ERESOURCE_ACCCOUNT_EXISTS),
        );
        assert!(
            account.sequence_number &#61;&#61; 0,
            error::invalid_state(EACCOUNT_ALREADY_USED),
        );
        create_signer(resource_addr)
    &#125; else &#123;
        create_account_unchecked(resource_addr)
    &#125;;

    // By default, only the SignerCapability should have control over the resource account and not the auth key.
    // If the source account wants direct control via auth key, they would need to explicitly rotate the auth key
    // of the resource account using the SignerCapability.
    rotate_authentication_key_internal(&amp;resource, ZERO_AUTH_KEY);

    let account &#61; borrow_global_mut&lt;Account&gt;(resource_addr);
    account.signer_capability_offer.for &#61; option::some(resource_addr);
    let signer_cap &#61; SignerCapability &#123; account: resource_addr &#125;;
    (resource, signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_account_create_framework_reserved_account"></a>

## Function `create_framework_reserved_account`

create the account for system reserved addresses


<pre><code>public(friend) fun create_framework_reserved_account(addr: address): (signer, account::SignerCapability)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun create_framework_reserved_account(addr: address): (signer, SignerCapability) &#123;
    assert!(
        addr &#61;&#61; @0x1 &#124;&#124;
            addr &#61;&#61; @0x2 &#124;&#124;
            addr &#61;&#61; @0x3 &#124;&#124;
            addr &#61;&#61; @0x4 &#124;&#124;
            addr &#61;&#61; @0x5 &#124;&#124;
            addr &#61;&#61; @0x6 &#124;&#124;
            addr &#61;&#61; @0x7 &#124;&#124;
            addr &#61;&#61; @0x8 &#124;&#124;
            addr &#61;&#61; @0x9 &#124;&#124;
            addr &#61;&#61; @0xa,
        error::permission_denied(ENO_VALID_FRAMEWORK_RESERVED_ADDRESS),
    );
    let signer &#61; create_account_unchecked(addr);
    let signer_cap &#61; SignerCapability &#123; account: addr &#125;;
    (signer, signer_cap)
&#125;
</code></pre>



</details>

<a id="0x1_account_create_guid"></a>

## Function `create_guid`

GUID management methods.


<pre><code>public fun create_guid(account_signer: &amp;signer): guid::GUID
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_guid(account_signer: &amp;signer): guid::GUID acquires Account &#123;
    let addr &#61; signer::address_of(account_signer);
    let account &#61; borrow_global_mut&lt;Account&gt;(addr);
    let guid &#61; guid::create(addr, &amp;mut account.guid_creation_num);
    assert!(
        account.guid_creation_num &lt; MAX_GUID_CREATION_NUM,
        error::out_of_range(EEXCEEDED_MAX_GUID_CREATION_NUM),
    );
    guid
&#125;
</code></pre>



</details>

<a id="0x1_account_new_event_handle"></a>

## Function `new_event_handle`

GUID management methods.


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(account: &amp;signer): event::EventHandle&lt;T&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_event_handle&lt;T: drop &#43; store&gt;(account: &amp;signer): EventHandle&lt;T&gt; acquires Account &#123;
    event::new_event_handle(create_guid(account))
&#125;
</code></pre>



</details>

<a id="0x1_account_register_coin"></a>

## Function `register_coin`

Coin management methods.


<pre><code>public(friend) fun register_coin&lt;CoinType&gt;(account_addr: address)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun register_coin&lt;CoinType&gt;(account_addr: address) acquires Account &#123;
    let account &#61; borrow_global_mut&lt;Account&gt;(account_addr);
    event::emit_event&lt;CoinRegisterEvent&gt;(
        &amp;mut account.coin_register_events,
        CoinRegisterEvent &#123;
            type_info: type_info::type_of&lt;CoinType&gt;(),
        &#125;,
    );
&#125;
</code></pre>



</details>

<a id="0x1_account_create_signer_with_capability"></a>

## Function `create_signer_with_capability`

Capability based functions for efficient use.


<pre><code>public fun create_signer_with_capability(capability: &amp;account::SignerCapability): signer
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun create_signer_with_capability(capability: &amp;SignerCapability): signer &#123;
    let addr &#61; &amp;capability.account;
    create_signer(&#42;addr)
&#125;
</code></pre>



</details>

<a id="0x1_account_get_signer_capability_address"></a>

## Function `get_signer_capability_address`



<pre><code>public fun get_signer_capability_address(capability: &amp;account::SignerCapability): address
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun get_signer_capability_address(capability: &amp;SignerCapability): address &#123;
    capability.account
&#125;
</code></pre>



</details>

<a id="0x1_account_verify_signed_message"></a>

## Function `verify_signed_message`



<pre><code>public fun verify_signed_message&lt;T: drop&gt;(account: address, account_scheme: u8, account_public_key: vector&lt;u8&gt;, signed_message_bytes: vector&lt;u8&gt;, message: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun verify_signed_message&lt;T: drop&gt;(
    account: address,
    account_scheme: u8,
    account_public_key: vector&lt;u8&gt;,
    signed_message_bytes: vector&lt;u8&gt;,
    message: T,
) acquires Account &#123;
    let account_resource &#61; borrow_global_mut&lt;Account&gt;(account);
    // Verify that the `SignerCapabilityOfferProofChallengeV2` has the right information and is signed by the account owner&apos;s key
    if (account_scheme &#61;&#61; ED25519_SCHEME) &#123;
        let pubkey &#61; ed25519::new_unvalidated_public_key_from_bytes(account_public_key);
        let expected_auth_key &#61; ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);
        assert!(
            account_resource.authentication_key &#61;&#61; expected_auth_key,
            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY),
        );

        let signer_capability_sig &#61; ed25519::new_signature_from_bytes(signed_message_bytes);
        assert!(
            ed25519::signature_verify_strict_t(&amp;signer_capability_sig, &amp;pubkey, message),
            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE),
        );
    &#125; else if (account_scheme &#61;&#61; MULTI_ED25519_SCHEME) &#123;
        let pubkey &#61; multi_ed25519::new_unvalidated_public_key_from_bytes(account_public_key);
        let expected_auth_key &#61; multi_ed25519::unvalidated_public_key_to_authentication_key(&amp;pubkey);
        assert!(
            account_resource.authentication_key &#61;&#61; expected_auth_key,
            error::invalid_argument(EWRONG_CURRENT_PUBLIC_KEY),
        );

        let signer_capability_sig &#61; multi_ed25519::new_signature_from_bytes(signed_message_bytes);
        assert!(
            multi_ed25519::signature_verify_strict_t(&amp;signer_capability_sig, &amp;pubkey, message),
            error::invalid_argument(EINVALID_PROOF_OF_KNOWLEDGE),
        );
    &#125; else &#123;
        abort error::invalid_argument(EINVALID_SCHEME)
    &#125;;
&#125;
</code></pre>



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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer)
</code></pre>


Only the address <code>@aptos_framework</code> can call.
OriginatingAddress does not exist under <code>@aptos_framework</code> before the call.


<pre><code>let aptos_addr &#61; signer::address_of(aptos_framework);
aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);
aborts_if exists&lt;OriginatingAddress&gt;(aptos_addr);
ensures exists&lt;OriginatingAddress&gt;(aptos_addr);
</code></pre>



<a id="@Specification_1_create_account_if_does_not_exist"></a>

### Function `create_account_if_does_not_exist`


<<<<<<< HEAD
<pre><code><b>public</b> <b>fun</b> <a href="account.md#0x1_account_create_account_if_does_not_exist">create_account_if_does_not_exist</a>(account_address: <b>address</b>)
=======
<pre><code>fun create_account_if_does_not_exist(account_address: address)
>>>>>>> 13c50e058f (support mdx)
</code></pre>


Ensure that the account exists at the end of the call.


<pre><code>let authentication_key &#61; bcs::to_bytes(account_address);
aborts_if !exists&lt;Account&gt;(account_address) &amp;&amp; (
    account_address &#61;&#61; @vm_reserved
    &#124;&#124; account_address &#61;&#61; @aptos_framework
    &#124;&#124; account_address &#61;&#61; @aptos_token
    &#124;&#124; !(len(authentication_key) &#61;&#61; 32)
);
ensures exists&lt;Account&gt;(account_address);
</code></pre>



<a id="@Specification_1_create_account"></a>

### Function `create_account`


<pre><code>public(friend) fun create_account(new_address: address): signer
</code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.
Limit the new account address is not @vm_reserved / @aptos_framework / @aptos_toke.


<pre><code>include CreateAccountAbortsIf &#123;addr: new_address&#125;;
aborts_if new_address &#61;&#61; @vm_reserved &#124;&#124; new_address &#61;&#61; @aptos_framework &#124;&#124; new_address &#61;&#61; @aptos_token;
ensures signer::address_of(result) &#61;&#61; new_address;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
ensures exists&lt;Account&gt;(new_address);
</code></pre>



<a id="@Specification_1_create_account_unchecked"></a>

### Function `create_account_unchecked`


<pre><code>fun create_account_unchecked(new_address: address): signer
</code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.


<pre><code>include CreateAccountAbortsIf &#123;addr: new_address&#125;;
ensures signer::address_of(result) &#61;&#61; new_address;
ensures exists&lt;Account&gt;(new_address);
</code></pre>



<a id="@Specification_1_exists_at"></a>

### Function `exists_at`


<pre><code>&#35;[view]
public fun exists_at(addr: address): bool
</code></pre>




<pre><code>// This enforces <a id="high-level-req-3" href="#high-level-req">high-level requirement 3</a>:
aborts_if false;
</code></pre>




<a id="0x1_account_CreateAccountAbortsIf"></a>


<pre><code>schema CreateAccountAbortsIf &#123;
    addr: address;
    let authentication_key &#61; bcs::to_bytes(addr);
    aborts_if len(authentication_key) !&#61; 32;
    aborts_if exists&lt;Account&gt;(addr);
    ensures len(authentication_key) &#61;&#61; 32;
&#125;
</code></pre>



<a id="@Specification_1_get_guid_next_creation_num"></a>

### Function `get_guid_next_creation_num`


<pre><code>&#35;[view]
public fun get_guid_next_creation_num(addr: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(addr);
ensures result &#61;&#61; global&lt;Account&gt;(addr).guid_creation_num;
</code></pre>



<a id="@Specification_1_get_sequence_number"></a>

### Function `get_sequence_number`


<pre><code>&#35;[view]
public fun get_sequence_number(addr: address): u64
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(addr);
ensures result &#61;&#61; global&lt;Account&gt;(addr).sequence_number;
</code></pre>



<a id="@Specification_1_increment_sequence_number"></a>

### Function `increment_sequence_number`


<pre><code>public(friend) fun increment_sequence_number(addr: address)
</code></pre>


The Account existed under the address.
The sequence_number of the Account is up to MAX_U64.


<pre><code>let sequence_number &#61; global&lt;Account&gt;(addr).sequence_number;
aborts_if !exists&lt;Account&gt;(addr);
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
aborts_if sequence_number &#61;&#61; MAX_U64;
modifies global&lt;Account&gt;(addr);
let post post_sequence_number &#61; global&lt;Account&gt;(addr).sequence_number;
ensures post_sequence_number &#61;&#61; sequence_number &#43; 1;
</code></pre>



<a id="@Specification_1_get_authentication_key"></a>

### Function `get_authentication_key`


<pre><code>&#35;[view]
public fun get_authentication_key(addr: address): vector&lt;u8&gt;
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(addr);
ensures result &#61;&#61; global&lt;Account&gt;(addr).authentication_key;
</code></pre>



<a id="@Specification_1_rotate_authentication_key_internal"></a>

### Function `rotate_authentication_key_internal`


<pre><code>public(friend) fun rotate_authentication_key_internal(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)
</code></pre>


The Account existed under the signer before the call.
The length of new_auth_key is 32.


<pre><code>let addr &#61; signer::address_of(account);
// This enforces <a id="high-level-req-10" href="#high-level-req">high-level requirement 10</a>:
let post account_resource &#61; global&lt;Account&gt;(addr);
aborts_if !exists&lt;Account&gt;(addr);
aborts_if vector::length(new_auth_key) !&#61; 32;
modifies global&lt;Account&gt;(addr);
ensures account_resource.authentication_key &#61;&#61; new_auth_key;
</code></pre>



<a id="@Specification_1_rotate_authentication_key_call"></a>

### Function `rotate_authentication_key_call`


<pre><code>entry fun rotate_authentication_key_call(account: &amp;signer, new_auth_key: vector&lt;u8&gt;)
</code></pre>




<pre><code>let addr &#61; signer::address_of(account);
// This enforces <a id="high-level-req-10" href="#high-level-req">high-level requirement 10</a>:
let post account_resource &#61; global&lt;Account&gt;(addr);
aborts_if !exists&lt;Account&gt;(addr);
aborts_if vector::length(new_auth_key) !&#61; 32;
modifies global&lt;Account&gt;(addr);
ensures account_resource.authentication_key &#61;&#61; new_auth_key;
</code></pre>




<a id="0x1_account_spec_assert_valid_rotation_proof_signature_and_get_auth_key"></a>


<pre><code>fun spec_assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector&lt;u8&gt;, signature: vector&lt;u8&gt;, challenge: RotationProofChallenge): vector&lt;u8&gt;;
</code></pre>



<a id="@Specification_1_rotate_authentication_key"></a>

### Function `rotate_authentication_key`


<pre><code>public entry fun rotate_authentication_key(account: &amp;signer, from_scheme: u8, from_public_key_bytes: vector&lt;u8&gt;, to_scheme: u8, to_public_key_bytes: vector&lt;u8&gt;, cap_rotate_key: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)
</code></pre>


The Account existed under the signer
The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME


<pre><code>let addr &#61; signer::address_of(account);
let account_resource &#61; global&lt;Account&gt;(addr);
aborts_if !exists&lt;Account&gt;(addr);
// This enforces <a id="high-level-req-6.1" href="#high-level-req">high-level requirement 6</a>:
include from_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: from_public_key_bytes &#125;;
aborts_if from_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;
    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);
    account_resource.authentication_key !&#61; expected_auth_key
&#125;);
include from_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: from_public_key_bytes &#125;;
aborts_if from_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;
    let from_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(from_public_key_bytes);
    account_resource.authentication_key !&#61; from_auth_key
&#125;);
// This enforces <a id="high-level-req-5.1" href="#high-level-req">high-level requirement 5</a>:
aborts_if from_scheme !&#61; ED25519_SCHEME &amp;&amp; from_scheme !&#61; MULTI_ED25519_SCHEME;
let curr_auth_key &#61; from_bcs::deserialize&lt;address&gt;(account_resource.authentication_key);
aborts_if !from_bcs::deserializable&lt;address&gt;(account_resource.authentication_key);
let challenge &#61; RotationProofChallenge &#123;
    sequence_number: account_resource.sequence_number,
    originator: addr,
    current_auth_key: curr_auth_key,
    new_public_key: to_public_key_bytes,
&#125;;
// This enforces <a id="high-level-req-9.1" href="#high-level-req">high-level requirement 9</a>:
include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;
    scheme: from_scheme,
    public_key_bytes: from_public_key_bytes,
    signature: cap_rotate_key,
    challenge,
&#125;;
include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;
    scheme: to_scheme,
    public_key_bytes: to_public_key_bytes,
    signature: cap_update_table,
    challenge,
&#125;;
let originating_addr &#61; addr;
let new_auth_key_vector &#61; spec_assert_valid_rotation_proof_signature_and_get_auth_key(to_scheme, to_public_key_bytes, cap_update_table, challenge);
let address_map &#61; global&lt;OriginatingAddress&gt;(@aptos_framework).address_map;
let new_auth_key &#61; from_bcs::deserialize&lt;address&gt;(new_auth_key_vector);
aborts_if !exists&lt;OriginatingAddress&gt;(@aptos_framework);
aborts_if !from_bcs::deserializable&lt;address&gt;(account_resource.authentication_key);
aborts_if table::spec_contains(address_map, curr_auth_key) &amp;&amp;
    table::spec_get(address_map, curr_auth_key) !&#61; originating_addr;
aborts_if !from_bcs::deserializable&lt;address&gt;(new_auth_key_vector);
aborts_if curr_auth_key !&#61; new_auth_key &amp;&amp; table::spec_contains(address_map, new_auth_key);
include UpdateAuthKeyAndOriginatingAddressTableAbortsIf &#123;
    originating_addr: addr,
&#125;;
let post auth_key &#61; global&lt;Account&gt;(addr).authentication_key;
ensures auth_key &#61;&#61; new_auth_key_vector;
</code></pre>



<a id="@Specification_1_rotate_authentication_key_with_rotation_capability"></a>

### Function `rotate_authentication_key_with_rotation_capability`


<pre><code>public entry fun rotate_authentication_key_with_rotation_capability(delegate_signer: &amp;signer, rotation_cap_offerer_address: address, new_scheme: u8, new_public_key_bytes: vector&lt;u8&gt;, cap_update_table: vector&lt;u8&gt;)
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(rotation_cap_offerer_address);
let delegate_address &#61; signer::address_of(delegate_signer);
let offerer_account_resource &#61; global&lt;Account&gt;(rotation_cap_offerer_address);
aborts_if !from_bcs::deserializable&lt;address&gt;(offerer_account_resource.authentication_key);
let curr_auth_key &#61; from_bcs::deserialize&lt;address&gt;(offerer_account_resource.authentication_key);
aborts_if !exists&lt;Account&gt;(delegate_address);
let challenge &#61; RotationProofChallenge &#123;
    sequence_number: global&lt;Account&gt;(delegate_address).sequence_number,
    originator: rotation_cap_offerer_address,
    current_auth_key: curr_auth_key,
    new_public_key: new_public_key_bytes,
&#125;;
// This enforces <a id="high-level-req-6.2" href="#high-level-req">high-level requirement 6</a>:
aborts_if !option::spec_contains(offerer_account_resource.rotation_capability_offer.for, delegate_address);
// This enforces <a id="high-level-req-9.1" href="#high-level-req">high-level requirement 9</a>:
include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;
    scheme: new_scheme,
    public_key_bytes: new_public_key_bytes,
    signature: cap_update_table,
    challenge,
&#125;;
let new_auth_key_vector &#61; spec_assert_valid_rotation_proof_signature_and_get_auth_key(new_scheme, new_public_key_bytes, cap_update_table, challenge);
let address_map &#61; global&lt;OriginatingAddress&gt;(@aptos_framework).address_map;
aborts_if !exists&lt;OriginatingAddress&gt;(@aptos_framework);
aborts_if !from_bcs::deserializable&lt;address&gt;(offerer_account_resource.authentication_key);
aborts_if table::spec_contains(address_map, curr_auth_key) &amp;&amp;
    table::spec_get(address_map, curr_auth_key) !&#61; rotation_cap_offerer_address;
aborts_if !from_bcs::deserializable&lt;address&gt;(new_auth_key_vector);
let new_auth_key &#61; from_bcs::deserialize&lt;address&gt;(new_auth_key_vector);
aborts_if curr_auth_key !&#61; new_auth_key &amp;&amp; table::spec_contains(address_map, new_auth_key);
include UpdateAuthKeyAndOriginatingAddressTableAbortsIf &#123;
    originating_addr: rotation_cap_offerer_address,
    account_resource: offerer_account_resource,
&#125;;
let post auth_key &#61; global&lt;Account&gt;(rotation_cap_offerer_address).authentication_key;
ensures auth_key &#61;&#61; new_auth_key_vector;
</code></pre>



<a id="@Specification_1_offer_rotation_capability"></a>

### Function `offer_rotation_capability`


<pre><code>public entry fun offer_rotation_capability(account: &amp;signer, rotation_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)
</code></pre>




<pre><code>let source_address &#61; signer::address_of(account);
let account_resource &#61; global&lt;Account&gt;(source_address);
let proof_challenge &#61; RotationCapabilityOfferProofChallengeV2 &#123;
    chain_id: global&lt;chain_id::ChainId&gt;(@aptos_framework).id,
    sequence_number: account_resource.sequence_number,
    source_address,
    recipient_address,
&#125;;
aborts_if !exists&lt;chain_id::ChainId&gt;(@aptos_framework);
aborts_if !exists&lt;Account&gt;(recipient_address);
aborts_if !exists&lt;Account&gt;(source_address);
include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;
aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;
    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
    account_resource.authentication_key !&#61; expected_auth_key
&#125;);
include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: rotation_capability_sig_bytes &#125;;
aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; !ed25519::spec_signature_verify_strict_t(
    ed25519::Signature &#123; bytes: rotation_capability_sig_bytes &#125;,
    ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,
    proof_challenge
);
include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;
aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;
    let expected_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
    account_resource.authentication_key !&#61; expected_auth_key
&#125;);
include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: rotation_capability_sig_bytes &#125;;
aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; !multi_ed25519::spec_signature_verify_strict_t(
    multi_ed25519::Signature &#123; bytes: rotation_capability_sig_bytes &#125;,
    multi_ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,
    proof_challenge
);
// This enforces <a id="high-level-req-5.2" href="#high-level-req">high-level requirement 5</a>:
aborts_if account_scheme !&#61; ED25519_SCHEME &amp;&amp; account_scheme !&#61; MULTI_ED25519_SCHEME;
// This enforces <a id="high-level-req-7.1" href="#high-level-req">high-level requirement 7</a>:
modifies global&lt;Account&gt;(source_address);
let post offer_for &#61; global&lt;Account&gt;(source_address).rotation_capability_offer.for;
ensures option::spec_borrow(offer_for) &#61;&#61; recipient_address;
</code></pre>



<a id="@Specification_1_is_rotation_capability_offered"></a>

### Function `is_rotation_capability_offered`


<pre><code>&#35;[view]
public fun is_rotation_capability_offered(account_addr: address): bool
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);
</code></pre>



<a id="@Specification_1_get_rotation_capability_offer_for"></a>

### Function `get_rotation_capability_offer_for`


<pre><code>&#35;[view]
public fun get_rotation_capability_offer_for(account_addr: address): address
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);
let account_resource &#61; global&lt;Account&gt;(account_addr);
aborts_if len(account_resource.rotation_capability_offer.for.vec) &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_revoke_rotation_capability"></a>

### Function `revoke_rotation_capability`


<pre><code>public entry fun revoke_rotation_capability(account: &amp;signer, to_be_revoked_address: address)
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(to_be_revoked_address);
let addr &#61; signer::address_of(account);
let account_resource &#61; global&lt;Account&gt;(addr);
aborts_if !exists&lt;Account&gt;(addr);
aborts_if !option::spec_contains(account_resource.rotation_capability_offer.for,to_be_revoked_address);
modifies global&lt;Account&gt;(addr);
ensures exists&lt;Account&gt;(to_be_revoked_address);
let post offer_for &#61; global&lt;Account&gt;(addr).rotation_capability_offer.for;
ensures !option::spec_is_some(offer_for);
</code></pre>



<a id="@Specification_1_revoke_any_rotation_capability"></a>

### Function `revoke_any_rotation_capability`


<pre><code>public entry fun revoke_any_rotation_capability(account: &amp;signer)
</code></pre>




<pre><code>let addr &#61; signer::address_of(account);
modifies global&lt;Account&gt;(addr);
aborts_if !exists&lt;Account&gt;(addr);
let account_resource &#61; global&lt;Account&gt;(addr);
// This enforces <a id="high-level-req-7.3" href="#high-level-req">high-level requirement 7</a>:
aborts_if !option::is_some(account_resource.rotation_capability_offer.for);
let post offer_for &#61; global&lt;Account&gt;(addr).rotation_capability_offer.for;
ensures !option::spec_is_some(offer_for);
</code></pre>



<a id="@Specification_1_offer_signer_capability"></a>

### Function `offer_signer_capability`


<pre><code>public entry fun offer_signer_capability(account: &amp;signer, signer_capability_sig_bytes: vector&lt;u8&gt;, account_scheme: u8, account_public_key_bytes: vector&lt;u8&gt;, recipient_address: address)
</code></pre>


The Account existed under the signer.
The authentication scheme is ED25519_SCHEME and MULTI_ED25519_SCHEME.


<pre><code>let source_address &#61; signer::address_of(account);
let account_resource &#61; global&lt;Account&gt;(source_address);
let proof_challenge &#61; SignerCapabilityOfferProofChallengeV2 &#123;
    sequence_number: account_resource.sequence_number,
    source_address,
    recipient_address,
&#125;;
aborts_if !exists&lt;Account&gt;(recipient_address);
aborts_if !exists&lt;Account&gt;(source_address);
include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;
aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;
    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
    account_resource.authentication_key !&#61; expected_auth_key
&#125;);
include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signer_capability_sig_bytes &#125;;
aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; !ed25519::spec_signature_verify_strict_t(
    ed25519::Signature &#123; bytes: signer_capability_sig_bytes &#125;,
    ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,
    proof_challenge
);
include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key_bytes &#125;;
aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;
    let expected_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key_bytes);
    account_resource.authentication_key !&#61; expected_auth_key
&#125;);
include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signer_capability_sig_bytes &#125;;
aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; !multi_ed25519::spec_signature_verify_strict_t(
    multi_ed25519::Signature &#123; bytes: signer_capability_sig_bytes &#125;,
    multi_ed25519::UnvalidatedPublicKey &#123; bytes: account_public_key_bytes &#125;,
    proof_challenge
);
// This enforces <a id="high-level-req-5.3" href="#high-level-req">high-level requirement 5</a>:
aborts_if account_scheme !&#61; ED25519_SCHEME &amp;&amp; account_scheme !&#61; MULTI_ED25519_SCHEME;
// This enforces <a id="high-level-req-7.2" href="#high-level-req">high-level requirement 7</a>:
modifies global&lt;Account&gt;(source_address);
let post offer_for &#61; global&lt;Account&gt;(source_address).signer_capability_offer.for;
ensures option::spec_borrow(offer_for) &#61;&#61; recipient_address;
</code></pre>



<a id="@Specification_1_is_signer_capability_offered"></a>

### Function `is_signer_capability_offered`


<pre><code>&#35;[view]
public fun is_signer_capability_offered(account_addr: address): bool
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);
</code></pre>



<a id="@Specification_1_get_signer_capability_offer_for"></a>

### Function `get_signer_capability_offer_for`


<pre><code>&#35;[view]
public fun get_signer_capability_offer_for(account_addr: address): address
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);
let account_resource &#61; global&lt;Account&gt;(account_addr);
aborts_if len(account_resource.signer_capability_offer.for.vec) &#61;&#61; 0;
</code></pre>



<a id="@Specification_1_revoke_signer_capability"></a>

### Function `revoke_signer_capability`


<pre><code>public entry fun revoke_signer_capability(account: &amp;signer, to_be_revoked_address: address)
</code></pre>


The Account existed under the signer.
The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address.


<pre><code>aborts_if !exists&lt;Account&gt;(to_be_revoked_address);
let addr &#61; signer::address_of(account);
let account_resource &#61; global&lt;Account&gt;(addr);
aborts_if !exists&lt;Account&gt;(addr);
aborts_if !option::spec_contains(account_resource.signer_capability_offer.for,to_be_revoked_address);
modifies global&lt;Account&gt;(addr);
ensures exists&lt;Account&gt;(to_be_revoked_address);
</code></pre>



<a id="@Specification_1_revoke_any_signer_capability"></a>

### Function `revoke_any_signer_capability`


<pre><code>public entry fun revoke_any_signer_capability(account: &amp;signer)
</code></pre>




<pre><code>modifies global&lt;Account&gt;(signer::address_of(account));
// This enforces <a id="high-level-req-7.4" href="#high-level-req">high-level requirement 7</a>:
aborts_if !exists&lt;Account&gt;(signer::address_of(account));
let account_resource &#61; global&lt;Account&gt;(signer::address_of(account));
aborts_if !option::is_some(account_resource.signer_capability_offer.for);
</code></pre>



<a id="@Specification_1_create_authorized_signer"></a>

### Function `create_authorized_signer`


<pre><code>public fun create_authorized_signer(account: &amp;signer, offerer_address: address): signer
</code></pre>


The Account existed under the signer.
The value of signer_capability_offer.for of Account resource under the signer is offerer_address.


<pre><code>// This enforces <a id="high-level-req-8" href="#high-level-req">high-level requirement 8</a>:
include AccountContainsAddr&#123;
    account,
    address: offerer_address,
&#125;;
modifies global&lt;Account&gt;(offerer_address);
ensures exists&lt;Account&gt;(offerer_address);
ensures signer::address_of(result) &#61;&#61; offerer_address;
</code></pre>




<a id="0x1_account_AccountContainsAddr"></a>


<pre><code>schema AccountContainsAddr &#123;
    account: signer;
    address: address;
    let addr &#61; signer::address_of(account);
    let account_resource &#61; global&lt;Account&gt;(address);
    aborts_if !exists&lt;Account&gt;(address);
    // This enforces <a id="high-level-spec-3" href="create_signer.md#high-level-req">high-level requirement 3</a> of the <a href=create_signer.md>create_signer</a> module:
    aborts_if !option::spec_contains(account_resource.signer_capability_offer.for,addr);
&#125;
</code></pre>



<a id="@Specification_1_assert_valid_rotation_proof_signature_and_get_auth_key"></a>

### Function `assert_valid_rotation_proof_signature_and_get_auth_key`


<pre><code>fun assert_valid_rotation_proof_signature_and_get_auth_key(scheme: u8, public_key_bytes: vector&lt;u8&gt;, signature: vector&lt;u8&gt;, challenge: &amp;account::RotationProofChallenge): vector&lt;u8&gt;
</code></pre>




<pre><code>pragma opaque;
include AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf;
ensures [abstract] result &#61;&#61; spec_assert_valid_rotation_proof_signature_and_get_auth_key(scheme, public_key_bytes, signature, challenge);
</code></pre>




<a id="0x1_account_AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf"></a>


<pre><code>schema AssertValidRotationProofSignatureAndGetAuthKeyAbortsIf &#123;
    scheme: u8;
    public_key_bytes: vector&lt;u8&gt;;
    signature: vector&lt;u8&gt;;
    challenge: RotationProofChallenge;
    include scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: public_key_bytes &#125;;
    include scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signature &#125;;
    aborts_if scheme &#61;&#61; ED25519_SCHEME &amp;&amp; !ed25519::spec_signature_verify_strict_t(
        ed25519::Signature &#123; bytes: signature &#125;,
        ed25519::UnvalidatedPublicKey &#123; bytes: public_key_bytes &#125;,
        challenge
    );
    include scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: public_key_bytes &#125;;
    include scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signature &#125;;
    aborts_if scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; !multi_ed25519::spec_signature_verify_strict_t(
        multi_ed25519::Signature &#123; bytes: signature &#125;,
        multi_ed25519::UnvalidatedPublicKey &#123; bytes: public_key_bytes &#125;,
        challenge
    );
    aborts_if scheme !&#61; ED25519_SCHEME &amp;&amp; scheme !&#61; MULTI_ED25519_SCHEME;
&#125;
</code></pre>



<a id="@Specification_1_update_auth_key_and_originating_address_table"></a>

### Function `update_auth_key_and_originating_address_table`


<pre><code>fun update_auth_key_and_originating_address_table(originating_addr: address, account_resource: &amp;mut account::Account, new_auth_key_vector: vector&lt;u8&gt;)
</code></pre>




<pre><code>modifies global&lt;OriginatingAddress&gt;(@aptos_framework);
include UpdateAuthKeyAndOriginatingAddressTableAbortsIf;
</code></pre>




<a id="0x1_account_UpdateAuthKeyAndOriginatingAddressTableAbortsIf"></a>


<pre><code>schema UpdateAuthKeyAndOriginatingAddressTableAbortsIf &#123;
    originating_addr: address;
    account_resource: Account;
    new_auth_key_vector: vector&lt;u8&gt;;
    let address_map &#61; global&lt;OriginatingAddress&gt;(@aptos_framework).address_map;
    let curr_auth_key &#61; from_bcs::deserialize&lt;address&gt;(account_resource.authentication_key);
    let new_auth_key &#61; from_bcs::deserialize&lt;address&gt;(new_auth_key_vector);
    aborts_if !exists&lt;OriginatingAddress&gt;(@aptos_framework);
    aborts_if !from_bcs::deserializable&lt;address&gt;(account_resource.authentication_key);
    aborts_if table::spec_contains(address_map, curr_auth_key) &amp;&amp;
        table::spec_get(address_map, curr_auth_key) !&#61; originating_addr;
    aborts_if !from_bcs::deserializable&lt;address&gt;(new_auth_key_vector);
    aborts_if curr_auth_key !&#61; new_auth_key &amp;&amp; table::spec_contains(address_map, new_auth_key);
    ensures table::spec_contains(global&lt;OriginatingAddress&gt;(@aptos_framework).address_map, from_bcs::deserialize&lt;address&gt;(new_auth_key_vector));
&#125;
</code></pre>



<a id="@Specification_1_create_resource_address"></a>

### Function `create_resource_address`


<pre><code>public fun create_resource_address(source: &amp;address, seed: vector&lt;u8&gt;): address
</code></pre>


The Account existed under the signer
The value of signer_capability_offer.for of Account resource under the signer is to_be_revoked_address


<pre><code>pragma opaque;
pragma aborts_if_is_strict &#61; false;
aborts_if [abstract] false;
ensures [abstract] result &#61;&#61; spec_create_resource_address(source, seed);
</code></pre>




<a id="0x1_account_spec_create_resource_address"></a>


<pre><code>fun spec_create_resource_address(source: address, seed: vector&lt;u8&gt;): address;
</code></pre>



<a id="@Specification_1_create_resource_account"></a>

### Function `create_resource_account`


<pre><code>public fun create_resource_account(source: &amp;signer, seed: vector&lt;u8&gt;): (signer, account::SignerCapability)
</code></pre>




<pre><code>let source_addr &#61; signer::address_of(source);
let resource_addr &#61; spec_create_resource_address(source_addr, seed);
aborts_if len(ZERO_AUTH_KEY) !&#61; 32;
include exists_at(resource_addr) &#61;&#61;&gt; CreateResourceAccountAbortsIf;
include !exists_at(resource_addr) &#61;&#61;&gt; CreateAccountAbortsIf &#123;addr: resource_addr&#125;;
ensures signer::address_of(result_1) &#61;&#61; resource_addr;
let post offer_for &#61; global&lt;Account&gt;(resource_addr).signer_capability_offer.for;
ensures option::spec_borrow(offer_for) &#61;&#61; resource_addr;
ensures result_2 &#61;&#61; SignerCapability &#123; account: resource_addr &#125;;
</code></pre>



<a id="@Specification_1_create_framework_reserved_account"></a>

### Function `create_framework_reserved_account`


<pre><code>public(friend) fun create_framework_reserved_account(addr: address): (signer, account::SignerCapability)
</code></pre>


Check if the bytes of the new address is 32.
The Account does not exist under the new address before creating the account.
The system reserved addresses is @0x1 / @0x2 / @0x3 / @0x4 / @0x5  / @0x6 / @0x7 / @0x8 / @0x9 / @0xa.


<pre><code>aborts_if spec_is_framework_address(addr);
include CreateAccountAbortsIf &#123;addr&#125;;
ensures signer::address_of(result_1) &#61;&#61; addr;
ensures result_2 &#61;&#61; SignerCapability &#123; account: addr &#125;;
</code></pre>




<a id="0x1_account_spec_is_framework_address"></a>


<pre><code>fun spec_is_framework_address(addr: address): bool&#123;
   addr !&#61; @0x1 &amp;&amp;
   addr !&#61; @0x2 &amp;&amp;
   addr !&#61; @0x3 &amp;&amp;
   addr !&#61; @0x4 &amp;&amp;
   addr !&#61; @0x5 &amp;&amp;
   addr !&#61; @0x6 &amp;&amp;
   addr !&#61; @0x7 &amp;&amp;
   addr !&#61; @0x8 &amp;&amp;
   addr !&#61; @0x9 &amp;&amp;
   addr !&#61; @0xa
&#125;
</code></pre>



<a id="@Specification_1_create_guid"></a>

### Function `create_guid`


<pre><code>public fun create_guid(account_signer: &amp;signer): guid::GUID
</code></pre>


The Account existed under the signer.
The guid_creation_num of the ccount resource is up to MAX_U64.


<pre><code>let addr &#61; signer::address_of(account_signer);
include NewEventHandleAbortsIf &#123;
    account: account_signer,
&#125;;
modifies global&lt;Account&gt;(addr);
// This enforces <a id="high-level-req-11" href="#high-level-req">high-level requirement 11</a>:
ensures global&lt;Account&gt;(addr).guid_creation_num &#61;&#61; old(global&lt;Account&gt;(addr).guid_creation_num) &#43; 1;
</code></pre>



<a id="@Specification_1_new_event_handle"></a>

### Function `new_event_handle`


<pre><code>public fun new_event_handle&lt;T: drop, store&gt;(account: &amp;signer): event::EventHandle&lt;T&gt;
</code></pre>


The Account existed under the signer.
The guid_creation_num of the Account is up to MAX_U64.


<pre><code>include NewEventHandleAbortsIf;
</code></pre>




<a id="0x1_account_NewEventHandleAbortsIf"></a>


<pre><code>schema NewEventHandleAbortsIf &#123;
    account: &amp;signer;
    let addr &#61; signer::address_of(account);
    let account &#61; global&lt;Account&gt;(addr);
    aborts_if !exists&lt;Account&gt;(addr);
    aborts_if account.guid_creation_num &#43; 1 &gt; MAX_U64;
    aborts_if account.guid_creation_num &#43; 1 &gt;&#61; MAX_GUID_CREATION_NUM;
&#125;
</code></pre>



<a id="@Specification_1_register_coin"></a>

### Function `register_coin`


<pre><code>public(friend) fun register_coin&lt;CoinType&gt;(account_addr: address)
</code></pre>




<pre><code>aborts_if !exists&lt;Account&gt;(account_addr);
aborts_if !type_info::spec_is_struct&lt;CoinType&gt;();
modifies global&lt;Account&gt;(account_addr);
</code></pre>



<a id="@Specification_1_create_signer_with_capability"></a>

### Function `create_signer_with_capability`


<pre><code>public fun create_signer_with_capability(capability: &amp;account::SignerCapability): signer
</code></pre>




<pre><code>let addr &#61; capability.account;
ensures signer::address_of(result) &#61;&#61; addr;
</code></pre>




<a id="0x1_account_CreateResourceAccountAbortsIf"></a>


<pre><code>schema CreateResourceAccountAbortsIf &#123;
    resource_addr: address;
    let account &#61; global&lt;Account&gt;(resource_addr);
    aborts_if len(account.signer_capability_offer.for.vec) !&#61; 0;
    aborts_if account.sequence_number !&#61; 0;
&#125;
</code></pre>



<a id="@Specification_1_verify_signed_message"></a>

### Function `verify_signed_message`


<pre><code>public fun verify_signed_message&lt;T: drop&gt;(account: address, account_scheme: u8, account_public_key: vector&lt;u8&gt;, signed_message_bytes: vector&lt;u8&gt;, message: T)
</code></pre>




<pre><code>pragma aborts_if_is_partial;
modifies global&lt;Account&gt;(account);
let account_resource &#61; global&lt;Account&gt;(account);
aborts_if !exists&lt;Account&gt;(account);
include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key &#125;;
aborts_if account_scheme &#61;&#61; ED25519_SCHEME &amp;&amp; (&#123;
    let expected_auth_key &#61; ed25519::spec_public_key_bytes_to_authentication_key(account_public_key);
    account_resource.authentication_key !&#61; expected_auth_key
&#125;);
include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewUnvalidatedPublicKeyFromBytesAbortsIf &#123; bytes: account_public_key &#125;;
aborts_if account_scheme &#61;&#61; MULTI_ED25519_SCHEME &amp;&amp; (&#123;
    let expected_auth_key &#61; multi_ed25519::spec_public_key_bytes_to_authentication_key(account_public_key);
    account_resource.authentication_key !&#61; expected_auth_key
&#125;);
include account_scheme &#61;&#61; ED25519_SCHEME &#61;&#61;&gt; ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signed_message_bytes &#125;;
include account_scheme &#61;&#61; MULTI_ED25519_SCHEME &#61;&#61;&gt; multi_ed25519::NewSignatureFromBytesAbortsIf &#123; bytes: signed_message_bytes &#125;;
aborts_if account_scheme !&#61; ED25519_SCHEME &amp;&amp; account_scheme !&#61; MULTI_ED25519_SCHEME;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
