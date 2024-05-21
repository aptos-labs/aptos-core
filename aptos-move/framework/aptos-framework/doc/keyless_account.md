
<a id="0x1_keyless_account"></a>

# Module `0x1::keyless_account`

This module is responsible for configuring keyless blockchain accounts which were introduced in
[AIP-61](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-61.md).


-  [Struct `Group`](#0x1_keyless_account_Group)
-  [Resource `Groth16VerificationKey`](#0x1_keyless_account_Groth16VerificationKey)
-  [Resource `Configuration`](#0x1_keyless_account_Configuration)
-  [Constants](#@Constants_0)
-  [Function `new_groth16_verification_key`](#0x1_keyless_account_new_groth16_verification_key)
-  [Function `new_configuration`](#0x1_keyless_account_new_configuration)
-  [Function `validate_groth16_vk`](#0x1_keyless_account_validate_groth16_vk)
-  [Function `update_groth16_verification_key`](#0x1_keyless_account_update_groth16_verification_key)
-  [Function `update_configuration`](#0x1_keyless_account_update_configuration)
-  [Function `update_training_wheels`](#0x1_keyless_account_update_training_wheels)
-  [Function `update_max_exp_horizon`](#0x1_keyless_account_update_max_exp_horizon)
-  [Function `remove_all_override_auds`](#0x1_keyless_account_remove_all_override_auds)
-  [Function `add_override_aud`](#0x1_keyless_account_add_override_aud)
-  [Function `set_groth16_verification_key_for_next_epoch`](#0x1_keyless_account_set_groth16_verification_key_for_next_epoch)
-  [Function `set_configuration_for_next_epoch`](#0x1_keyless_account_set_configuration_for_next_epoch)
-  [Function `update_training_wheels_for_next_epoch`](#0x1_keyless_account_update_training_wheels_for_next_epoch)
-  [Function `update_max_exp_horizon_for_next_epoch`](#0x1_keyless_account_update_max_exp_horizon_for_next_epoch)
-  [Function `remove_all_override_auds_for_next_epoch`](#0x1_keyless_account_remove_all_override_auds_for_next_epoch)
-  [Function `add_override_aud_for_next_epoch`](#0x1_keyless_account_add_override_aud_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_keyless_account_on_new_epoch)
-  [Specification](#@Specification_1)


<pre><code>use 0x1::bn254_algebra;
use 0x1::chain_status;
use 0x1::config_buffer;
use 0x1::crypto_algebra;
use 0x1::ed25519;
use 0x1::option;
use 0x1::signer;
use 0x1::string;
use 0x1::system_addresses;
</code></pre>



<a id="0x1_keyless_account_Group"></a>

## Struct `Group`



<pre><code>&#35;[resource_group(&#35;[scope &#61; global])]
struct Group
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_keyless_account_Groth16VerificationKey"></a>

## Resource `Groth16VerificationKey`

The 288-byte Groth16 verification key (VK) for the ZK relation that implements keyless accounts


<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::keyless_account::Group])]
struct Groth16VerificationKey has drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1: vector&lt;u8&gt;</code>
</dt>
<dd>
 32-byte serialization of <code>alpha &#42; G</code>, where <code>G</code> is the generator of <code>G1</code>.
</dd>
<dt>
<code>beta_g2: vector&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>alpha &#42; H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_g2: vector&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>gamma &#42; H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>delta_g2: vector&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>delta &#42; H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_abc_g1: vector&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
 <code>\forall i \in &#123;0, ..., \ell&#125;, 64&#45;byte serialization of gamma^&#123;&#45;1&#125; &#42; (beta &#42; a_i &#43; alpha &#42; b_i &#43; c_i) &#42; H</code>, where
 <code>H</code> is the generator of <code>G1</code> and <code>\ell</code> is 1 for the ZK relation.
</dd>
</dl>


</details>

<a id="0x1_keyless_account_Configuration"></a>

## Resource `Configuration`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; 0x1::keyless_account::Group])]
struct Configuration has copy, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>override_aud_vals: vector&lt;string::String&gt;</code>
</dt>
<dd>
 An override <code>aud</code> for the identity of a recovery service, which will help users recover their keyless accounts
 associated with dapps or wallets that have disappeared.
 IMPORTANT: This recovery service **cannot** on its own take over user accounts; a user must first sign in
 via OAuth in the recovery service in order to allow it to rotate any of that user's keyless accounts.
</dd>
<dt>
<code>max_signatures_per_txn: u16</code>
</dt>
<dd>
 No transaction can have more than this many keyless signatures.
</dd>
<dt>
<code>max_exp_horizon_secs: u64</code>
</dt>
<dd>
 How far in the future from the JWT issued at time the EPK expiry can be set.
</dd>
<dt>
<code>training_wheels_pubkey: option::Option&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
 The training wheels PK, if training wheels are on
</dd>
<dt>
<code>max_commited_epk_bytes: u16</code>
</dt>
<dd>
 The max length of an ephemeral public key supported in our circuit (93 bytes)
</dd>
<dt>
<code>max_iss_val_bytes: u16</code>
</dt>
<dd>
 The max length of the value of the JWT's <code>iss</code> field supported in our circuit (e.g., <code>&quot;https://accounts.google.com&quot;</code>)
</dd>
<dt>
<code>max_extra_field_bytes: u16</code>
</dt>
<dd>
 The max length of the JWT field name and value (e.g., <code>&quot;max_age&quot;:&quot;18&quot;</code>) supported in our circuit
</dd>
<dt>
<code>max_jwt_header_b64_bytes: u32</code>
</dt>
<dd>
 The max length of the base64url-encoded JWT header in bytes supported in our circuit
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION"></a>

A serialized BN254 G1 point is invalid.


<pre><code>const E_INVALID_BN254_G1_SERIALIZATION: u64 &#61; 2;
</code></pre>



<a id="0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION"></a>

A serialized BN254 G2 point is invalid.


<pre><code>const E_INVALID_BN254_G2_SERIALIZATION: u64 &#61; 3;
</code></pre>



<a id="0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE"></a>

The training wheels PK needs to be 32 bytes long.


<pre><code>const E_TRAINING_WHEELS_PK_WRONG_SIZE: u64 &#61; 1;
</code></pre>



<a id="0x1_keyless_account_new_groth16_verification_key"></a>

## Function `new_groth16_verification_key`



<pre><code>public fun new_groth16_verification_key(alpha_g1: vector&lt;u8&gt;, beta_g2: vector&lt;u8&gt;, gamma_g2: vector&lt;u8&gt;, delta_g2: vector&lt;u8&gt;, gamma_abc_g1: vector&lt;vector&lt;u8&gt;&gt;): keyless_account::Groth16VerificationKey
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_groth16_verification_key(alpha_g1: vector&lt;u8&gt;,
                                        beta_g2: vector&lt;u8&gt;,
                                        gamma_g2: vector&lt;u8&gt;,
                                        delta_g2: vector&lt;u8&gt;,
                                        gamma_abc_g1: vector&lt;vector&lt;u8&gt;&gt;
): Groth16VerificationKey &#123;
    Groth16VerificationKey &#123;
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_new_configuration"></a>

## Function `new_configuration`



<pre><code>public fun new_configuration(override_aud_val: vector&lt;string::String&gt;, max_signatures_per_txn: u16, max_exp_horizon_secs: u64, training_wheels_pubkey: option::Option&lt;vector&lt;u8&gt;&gt;, max_commited_epk_bytes: u16, max_iss_val_bytes: u16, max_extra_field_bytes: u16, max_jwt_header_b64_bytes: u32): keyless_account::Configuration
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun new_configuration(
    override_aud_val: vector&lt;String&gt;,
    max_signatures_per_txn: u16,
    max_exp_horizon_secs: u64,
    training_wheels_pubkey: Option&lt;vector&lt;u8&gt;&gt;,
    max_commited_epk_bytes: u16,
    max_iss_val_bytes: u16,
    max_extra_field_bytes: u16,
    max_jwt_header_b64_bytes: u32
): Configuration &#123;
    Configuration &#123;
        override_aud_vals: override_aud_val,
        max_signatures_per_txn,
        max_exp_horizon_secs,
        training_wheels_pubkey,
        max_commited_epk_bytes,
        max_iss_val_bytes,
        max_extra_field_bytes,
        max_jwt_header_b64_bytes,
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_validate_groth16_vk"></a>

## Function `validate_groth16_vk`

Pre-validate the VK to actively-prevent incorrect VKs from being set on-chain.


<pre><code>fun validate_groth16_vk(vk: &amp;keyless_account::Groth16VerificationKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun validate_groth16_vk(vk: &amp;Groth16VerificationKey) &#123;
    // Could be leveraged to speed up the VM deserialization of the VK by 2x, since it can assume the points are valid.
    assert!(option::is_some(&amp;crypto_algebra::deserialize&lt;bn254_algebra::G1, bn254_algebra::FormatG1Compr&gt;(&amp;vk.alpha_g1)), E_INVALID_BN254_G1_SERIALIZATION);
    assert!(option::is_some(&amp;crypto_algebra::deserialize&lt;bn254_algebra::G2, bn254_algebra::FormatG2Compr&gt;(&amp;vk.beta_g2)), E_INVALID_BN254_G2_SERIALIZATION);
    assert!(option::is_some(&amp;crypto_algebra::deserialize&lt;bn254_algebra::G2, bn254_algebra::FormatG2Compr&gt;(&amp;vk.gamma_g2)), E_INVALID_BN254_G2_SERIALIZATION);
    assert!(option::is_some(&amp;crypto_algebra::deserialize&lt;bn254_algebra::G2, bn254_algebra::FormatG2Compr&gt;(&amp;vk.delta_g2)), E_INVALID_BN254_G2_SERIALIZATION);
    for (i in 0..vector::length(&amp;vk.gamma_abc_g1)) &#123;
        assert!(option::is_some(&amp;crypto_algebra::deserialize&lt;bn254_algebra::G1, bn254_algebra::FormatG1Compr&gt;(vector::borrow(&amp;vk.gamma_abc_g1, i))), E_INVALID_BN254_G1_SERIALIZATION);
    &#125;;
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_update_groth16_verification_key"></a>

## Function `update_groth16_verification_key`

Sets the Groth16 verification key, only callable during genesis. To call during governance proposals, use
<code>set_groth16_verification_key_for_next_epoch</code>.

WARNING: See <code>set_groth16_verification_key_for_next_epoch</code> for caveats.


<pre><code>public fun update_groth16_verification_key(fx: &amp;signer, vk: keyless_account::Groth16VerificationKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_groth16_verification_key(fx: &amp;signer, vk: Groth16VerificationKey) &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();
    // There should not be a previous resource set here.
    move_to(fx, vk);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_update_configuration"></a>

## Function `update_configuration`

Sets the keyless configuration, only callable during genesis. To call during governance proposals, use
<code>set_configuration_for_next_epoch</code>.

WARNING: See <code>set_configuration_for_next_epoch</code> for caveats.


<pre><code>public fun update_configuration(fx: &amp;signer, config: keyless_account::Configuration)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_configuration(fx: &amp;signer, config: Configuration) &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();
    // There should not be a previous resource set here.
    move_to(fx, config);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_update_training_wheels"></a>

## Function `update_training_wheels`



<pre><code>&#35;[deprecated]
public fun update_training_wheels(fx: &amp;signer, pk: option::Option&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_training_wheels(fx: &amp;signer, pk: Option&lt;vector&lt;u8&gt;&gt;) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();

    if (option::is_some(&amp;pk)) &#123;
        assert!(vector::length(option::borrow(&amp;pk)) &#61;&#61; 32, E_TRAINING_WHEELS_PK_WRONG_SIZE)
    &#125;;

    let config &#61; borrow_global_mut&lt;Configuration&gt;(signer::address_of(fx));
    config.training_wheels_pubkey &#61; pk;
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_update_max_exp_horizon"></a>

## Function `update_max_exp_horizon`



<pre><code>&#35;[deprecated]
public fun update_max_exp_horizon(fx: &amp;signer, max_exp_horizon_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_max_exp_horizon(fx: &amp;signer, max_exp_horizon_secs: u64) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();

    let config &#61; borrow_global_mut&lt;Configuration&gt;(signer::address_of(fx));
    config.max_exp_horizon_secs &#61; max_exp_horizon_secs;
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_remove_all_override_auds"></a>

## Function `remove_all_override_auds`



<pre><code>&#35;[deprecated]
public fun remove_all_override_auds(fx: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_all_override_auds(fx: &amp;signer) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();

    let config &#61; borrow_global_mut&lt;Configuration&gt;(signer::address_of(fx));
    config.override_aud_vals &#61; vector[];
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_add_override_aud"></a>

## Function `add_override_aud`



<pre><code>&#35;[deprecated]
public fun add_override_aud(fx: &amp;signer, aud: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_override_aud(fx: &amp;signer, aud: String) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);
    chain_status::assert_genesis();

    let config &#61; borrow_global_mut&lt;Configuration&gt;(signer::address_of(fx));
    vector::push_back(&amp;mut config.override_aud_vals, aud);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_set_groth16_verification_key_for_next_epoch"></a>

## Function `set_groth16_verification_key_for_next_epoch`

Queues up a change to the Groth16 verification key. The change will only be effective after reconfiguration.
Only callable via governance proposal.

WARNING: To mitigate against DoS attacks, a VK change should be done together with a training wheels PK change,
so that old ZKPs for the old VK cannot be replayed as potentially-valid ZKPs.

WARNING: If a malicious key is set, this would lead to stolen funds.


<pre><code>public fun set_groth16_verification_key_for_next_epoch(fx: &amp;signer, vk: keyless_account::Groth16VerificationKey)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_groth16_verification_key_for_next_epoch(fx: &amp;signer, vk: Groth16VerificationKey) &#123;
    system_addresses::assert_aptos_framework(fx);
    validate_groth16_vk(&amp;vk);
    config_buffer::upsert&lt;Groth16VerificationKey&gt;(vk);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_set_configuration_for_next_epoch"></a>

## Function `set_configuration_for_next_epoch`

Queues up a change to the keyless configuration. The change will only be effective after reconfiguration. Only
callable via governance proposal.

WARNING: A malicious <code>Configuration</code> could lead to DoS attacks, create liveness issues, or enable a malicious
recovery service provider to phish users' accounts.


<pre><code>public fun set_configuration_for_next_epoch(fx: &amp;signer, config: keyless_account::Configuration)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_configuration_for_next_epoch(fx: &amp;signer, config: Configuration) &#123;
    system_addresses::assert_aptos_framework(fx);
    config_buffer::upsert&lt;Configuration&gt;(config);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_update_training_wheels_for_next_epoch"></a>

## Function `update_training_wheels_for_next_epoch`

Convenience method to queue up a change to the training wheels PK. The change will only be effective after
reconfiguration. Only callable via governance proposal.

WARNING: If a malicious key is set, this *could* lead to stolen funds.


<pre><code>public fun update_training_wheels_for_next_epoch(fx: &amp;signer, pk: option::Option&lt;vector&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_training_wheels_for_next_epoch(fx: &amp;signer, pk: Option&lt;vector&lt;u8&gt;&gt;) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);

    // If a PK is being set, validate it first.
    if (option::is_some(&amp;pk)) &#123;
        let bytes &#61; &#42;option::borrow(&amp;pk);
        let vpk &#61; ed25519::new_validated_public_key_from_bytes(bytes);
        assert!(option::is_some(&amp;vpk), E_TRAINING_WHEELS_PK_WRONG_SIZE)
    &#125;;

    let config &#61; if (config_buffer::does_exist&lt;Configuration&gt;()) &#123;
        config_buffer::extract&lt;Configuration&gt;()
    &#125; else &#123;
        &#42;borrow_global&lt;Configuration&gt;(signer::address_of(fx))
    &#125;;

    config.training_wheels_pubkey &#61; pk;

    set_configuration_for_next_epoch(fx, config);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_update_max_exp_horizon_for_next_epoch"></a>

## Function `update_max_exp_horizon_for_next_epoch`

Convenience method to queues up a change to the max expiration horizon. The change will only be effective after
reconfiguration. Only callable via governance proposal.


<pre><code>public fun update_max_exp_horizon_for_next_epoch(fx: &amp;signer, max_exp_horizon_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun update_max_exp_horizon_for_next_epoch(fx: &amp;signer, max_exp_horizon_secs: u64) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);

    let config &#61; if (config_buffer::does_exist&lt;Configuration&gt;()) &#123;
        config_buffer::extract&lt;Configuration&gt;()
    &#125; else &#123;
        &#42;borrow_global&lt;Configuration&gt;(signer::address_of(fx))
    &#125;;

    config.max_exp_horizon_secs &#61; max_exp_horizon_secs;

    set_configuration_for_next_epoch(fx, config);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_remove_all_override_auds_for_next_epoch"></a>

## Function `remove_all_override_auds_for_next_epoch`

Convenience method to queue up clearing the set of override <code>aud</code>'s. The change will only be effective after
reconfiguration. Only callable via governance proposal.

WARNING: When no override <code>aud</code> is set, recovery of keyless accounts associated with applications that disappeared
is no longer possible.


<pre><code>public fun remove_all_override_auds_for_next_epoch(fx: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun remove_all_override_auds_for_next_epoch(fx: &amp;signer) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);

    let config &#61; if (config_buffer::does_exist&lt;Configuration&gt;()) &#123;
        config_buffer::extract&lt;Configuration&gt;()
    &#125; else &#123;
        &#42;borrow_global&lt;Configuration&gt;(signer::address_of(fx))
    &#125;;

    config.override_aud_vals &#61; vector[];

    set_configuration_for_next_epoch(fx, config);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_add_override_aud_for_next_epoch"></a>

## Function `add_override_aud_for_next_epoch`

Convenience method to queue up an append to to the set of override <code>aud</code>'s. The change will only be effective
after reconfiguration. Only callable via governance proposal.

WARNING: If a malicious override <code>aud</code> is set, this *could* lead to stolen funds.


<pre><code>public fun add_override_aud_for_next_epoch(fx: &amp;signer, aud: string::String)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun add_override_aud_for_next_epoch(fx: &amp;signer, aud: String) acquires Configuration &#123;
    system_addresses::assert_aptos_framework(fx);

    let config &#61; if (config_buffer::does_exist&lt;Configuration&gt;()) &#123;
        config_buffer::extract&lt;Configuration&gt;()
    &#125; else &#123;
        &#42;borrow_global&lt;Configuration&gt;(signer::address_of(fx))
    &#125;;

    vector::push_back(&amp;mut config.override_aud_vals, aud);

    set_configuration_for_next_epoch(fx, config);
&#125;
</code></pre>



</details>

<a id="0x1_keyless_account_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the queued up configuration changes, if there are any.


<pre><code>public(friend) fun on_new_epoch(fx: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(fx: &amp;signer) acquires Groth16VerificationKey, Configuration &#123;
    system_addresses::assert_aptos_framework(fx);

    if (config_buffer::does_exist&lt;Groth16VerificationKey&gt;()) &#123;
        let vk &#61; config_buffer::extract();
        if (exists&lt;Groth16VerificationKey&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;Groth16VerificationKey&gt;(@aptos_framework) &#61; vk;
        &#125; else &#123;
            move_to(fx, vk);
        &#125;
    &#125;;

    if (config_buffer::does_exist&lt;Configuration&gt;()) &#123;
        let config &#61; config_buffer::extract();
        if (exists&lt;Configuration&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;Configuration&gt;(@aptos_framework) &#61; config;
        &#125; else &#123;
            move_to(fx, config);
        &#125;
    &#125;;
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify&#61;false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
