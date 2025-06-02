
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


<pre><code><b>use</b> <a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra">0x1::bn254_algebra</a>;
<b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;
<b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="../../../aptos-stdlib/tests/compiler-v2-doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="../../../aptos-stdlib/tests/compiler-v2-doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_keyless_account_Group"></a>

## Struct `Group`



<pre><code>#[resource_group(#[scope = <b>global</b>])]
<b>struct</b> <a href="keyless_account.md#0x1_keyless_account_Group">Group</a>
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


<pre><code>#[resource_group_member(#[group = <a href="keyless_account.md#0x1_keyless_account_Group">0x1::keyless_account::Group</a>])]
<b>struct</b> <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a> <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 32-byte serialization of <code>alpha * G</code>, where <code>G</code> is the generator of <code>G1</code>.
</dd>
<dt>
<code>beta_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>alpha * H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>gamma * H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>delta_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>delta * H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_abc_g1: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 <code>\<b>forall</b> i \in {0, ..., \ell}, 64-byte serialization of gamma^{-1} * (beta * a_i + alpha * b_i + c_i) * H</code>, where
 <code>H</code> is the generator of <code>G1</code> and <code>\ell</code> is 1 for the ZK relation.
</dd>
</dl>


</details>

<a id="0x1_keyless_account_Configuration"></a>

## Resource `Configuration`



<pre><code>#[resource_group_member(#[group = <a href="keyless_account.md#0x1_keyless_account_Group">0x1::keyless_account::Group</a>])]
<b>struct</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>override_aud_vals: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>&gt;</code>
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
<code>training_wheels_pubkey: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
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
 The max length of the value of the JWT's <code>iss</code> field supported in our circuit (e.g., <code>"https://accounts.google.com"</code>)
</dd>
<dt>
<code>max_extra_field_bytes: u16</code>
</dt>
<dd>
 The max length of the JWT field name and value (e.g., <code>"max_age":"18"</code>) supported in our circuit
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


<pre><code><b>const</b> <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION">E_INVALID_BN254_G1_SERIALIZATION</a>: u64 = 2;
</code></pre>



<a id="0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION"></a>

A serialized BN254 G2 point is invalid.


<pre><code><b>const</b> <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>: u64 = 3;
</code></pre>



<a id="0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE"></a>

The training wheels PK needs to be 32 bytes long.


<pre><code><b>const</b> <a href="keyless_account.md#0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>: u64 = 1;
</code></pre>



<a id="0x1_keyless_account_new_groth16_verification_key"></a>

## Function `new_groth16_verification_key`



<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, beta_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, delta_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_abc_g1: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        beta_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        gamma_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        delta_g2: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        gamma_abc_g1: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
): <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a> {
    <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a> {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    }
}
</code></pre>



</details>

<a id="0x1_keyless_account_new_configuration"></a>

## Function `new_configuration`



<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_configuration">new_configuration</a>(override_aud_val: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>&gt;, max_signatures_per_txn: u16, max_exp_horizon_secs: u64, training_wheels_pubkey: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, max_commited_epk_bytes: u16, max_iss_val_bytes: u16, max_extra_field_bytes: u16, max_jwt_header_b64_bytes: u32): <a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_configuration">new_configuration</a>(
    override_aud_val: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    max_signatures_per_txn: u16,
    max_exp_horizon_secs: u64,
    training_wheels_pubkey: Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    max_commited_epk_bytes: u16,
    max_iss_val_bytes: u16,
    max_extra_field_bytes: u16,
    max_jwt_header_b64_bytes: u32
): <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
        override_aud_vals: override_aud_val,
        max_signatures_per_txn,
        max_exp_horizon_secs,
        training_wheels_pubkey,
        max_commited_epk_bytes,
        max_iss_val_bytes,
        max_extra_field_bytes,
        max_jwt_header_b64_bytes,
    }
}
</code></pre>



</details>

<a id="0x1_keyless_account_validate_groth16_vk"></a>

## Function `validate_groth16_vk`

Pre-validate the VK to actively-prevent incorrect VKs from being set on-chain.


<pre><code><b>fun</b> <a href="keyless_account.md#0x1_keyless_account_validate_groth16_vk">validate_groth16_vk</a>(vk: &<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="keyless_account.md#0x1_keyless_account_validate_groth16_vk">validate_groth16_vk</a>(vk: &<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>) {
    // Could be leveraged <b>to</b> speed up the VM deserialization of the VK by 2x, since it can <b>assume</b> the points are valid.
    <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="../../../aptos-stdlib/tests/compiler-v2-doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_G1">bn254_algebra::G1</a>, <a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_FormatG1Compr">bn254_algebra::FormatG1Compr</a>&gt;(&vk.alpha_g1)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION">E_INVALID_BN254_G1_SERIALIZATION</a>);
    <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="../../../aptos-stdlib/tests/compiler-v2-doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_G2">bn254_algebra::G2</a>, <a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_FormatG2Compr">bn254_algebra::FormatG2Compr</a>&gt;(&vk.beta_g2)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>);
    <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="../../../aptos-stdlib/tests/compiler-v2-doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_G2">bn254_algebra::G2</a>, <a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_FormatG2Compr">bn254_algebra::FormatG2Compr</a>&gt;(&vk.gamma_g2)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>);
    <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="../../../aptos-stdlib/tests/compiler-v2-doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_G2">bn254_algebra::G2</a>, <a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_FormatG2Compr">bn254_algebra::FormatG2Compr</a>&gt;(&vk.delta_g2)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>);
    for (i in 0..<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(&vk.gamma_abc_g1)) {
        <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&<a href="../../../aptos-stdlib/tests/compiler-v2-doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_G1">bn254_algebra::G1</a>, <a href="../../../aptos-stdlib/tests/compiler-v2-doc/bn254_algebra.md#0x1_bn254_algebra_FormatG1Compr">bn254_algebra::FormatG1Compr</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&vk.gamma_abc_g1, i))), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION">E_INVALID_BN254_G1_SERIALIZATION</a>);
    };
}
</code></pre>



</details>

<a id="0x1_keyless_account_update_groth16_verification_key"></a>

## Function `update_groth16_verification_key`

Sets the Groth16 verification key, only callable during genesis. To call during governance proposals, use
<code>set_groth16_verification_key_for_next_epoch</code>.

WARNING: See <code>set_groth16_verification_key_for_next_epoch</code> for caveats.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_groth16_verification_key">update_groth16_verification_key</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_groth16_verification_key">update_groth16_verification_key</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();
    // There should not be a previous resource set here.
    <b>move_to</b>(fx, vk);
}
</code></pre>



</details>

<a id="0x1_keyless_account_update_configuration"></a>

## Function `update_configuration`

Sets the keyless configuration, only callable during genesis. To call during governance proposals, use
<code>set_configuration_for_next_epoch</code>.

WARNING: See <code>set_configuration_for_next_epoch</code> for caveats.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_configuration">update_configuration</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_configuration">update_configuration</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();
    // There should not be a previous resource set here.
    <b>move_to</b>(fx, config);
}
</code></pre>



</details>

<a id="0x1_keyless_account_update_training_wheels"></a>

## Function `update_training_wheels`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels">update_training_wheels</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels">update_training_wheels</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, pk: Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();

    <b>if</b> (<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&pk)) {
        <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_length">vector::length</a>(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&pk)) == 32, <a href="keyless_account.md#0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>)
    };

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    config.training_wheels_pubkey = pk;
}
</code></pre>



</details>

<a id="0x1_keyless_account_update_max_exp_horizon"></a>

## Function `update_max_exp_horizon`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon">update_max_exp_horizon</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon">update_max_exp_horizon</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    config.max_exp_horizon_secs = max_exp_horizon_secs;
}
</code></pre>



</details>

<a id="0x1_keyless_account_remove_all_override_auds"></a>

## Function `remove_all_override_auds`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds">remove_all_override_auds</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds">remove_all_override_auds</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    config.override_aud_vals = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[];
}
</code></pre>



</details>

<a id="0x1_keyless_account_add_override_aud"></a>

## Function `add_override_aud`



<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud">add_override_aud</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, aud: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud">add_override_aud</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, aud: String) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> config.override_aud_vals, aud);
}
</code></pre>



</details>

<a id="0x1_keyless_account_set_groth16_verification_key_for_next_epoch"></a>

## Function `set_groth16_verification_key_for_next_epoch`

Queues up a change to the Groth16 verification key. The change will only be effective after reconfiguration.
Only callable via governance proposal.

WARNING: To mitigate against DoS attacks, a VK change should be done together with a training wheels PK change,
so that old ZKPs for the old VK cannot be replayed as potentially-valid ZKPs.

WARNING: If a malicious key is set, this would lead to stolen funds.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_groth16_verification_key_for_next_epoch">set_groth16_verification_key_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_groth16_verification_key_for_next_epoch">set_groth16_verification_key_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(vk);
}
</code></pre>



</details>

<a id="0x1_keyless_account_set_configuration_for_next_epoch"></a>

## Function `set_configuration_for_next_epoch`

Queues up a change to the keyless configuration. The change will only be effective after reconfiguration. Only
callable via governance proposal.

WARNING: A malicious <code><a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a></code> could lead to DoS attacks, create liveness issues, or enable a malicious
recovery service provider to phish users' accounts.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(config);
}
</code></pre>



</details>

<a id="0x1_keyless_account_update_training_wheels_for_next_epoch"></a>

## Function `update_training_wheels_for_next_epoch`

Convenience method to queue up a change to the training wheels PK. The change will only be effective after
reconfiguration. Only callable via governance proposal.

WARNING: If a malicious key is set, this *could* lead to stolen funds.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels_for_next_epoch">update_training_wheels_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels_for_next_epoch">update_training_wheels_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, pk: Option&lt;<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    // If a PK is being set, validate it first.
    <b>if</b> (<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&pk)) {
        <b>let</b> bytes = *<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_borrow">option::borrow</a>(&pk);
        <b>let</b> vpk = <a href="../../../aptos-stdlib/tests/compiler-v2-doc/ed25519.md#0x1_ed25519_new_validated_public_key_from_bytes">ed25519::new_validated_public_key_from_bytes</a>(bytes);
        <b>assert</b>!(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/option.md#0x1_option_is_some">option::is_some</a>(&vpk), <a href="keyless_account.md#0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>)
    };

    <b>let</b> config = <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) {
        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()
    } <b>else</b> {
        *<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))
    };

    config.training_wheels_pubkey = pk;

    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);
}
</code></pre>



</details>

<a id="0x1_keyless_account_update_max_exp_horizon_for_next_epoch"></a>

## Function `update_max_exp_horizon_for_next_epoch`

Convenience method to queues up a change to the max expiration horizon. The change will only be effective after
reconfiguration. Only callable via governance proposal.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon_for_next_epoch">update_max_exp_horizon_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon_for_next_epoch">update_max_exp_horizon_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>let</b> config = <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) {
        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()
    } <b>else</b> {
        *<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))
    };

    config.max_exp_horizon_secs = max_exp_horizon_secs;

    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);
}
</code></pre>



</details>

<a id="0x1_keyless_account_remove_all_override_auds_for_next_epoch"></a>

## Function `remove_all_override_auds_for_next_epoch`

Convenience method to queue up clearing the set of override <code>aud</code>'s. The change will only be effective after
reconfiguration. Only callable via governance proposal.

WARNING: When no override <code>aud</code> is set, recovery of keyless accounts associated with applications that disappeared
is no longer possible.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds_for_next_epoch">remove_all_override_auds_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds_for_next_epoch">remove_all_override_auds_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>let</b> config = <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) {
        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()
    } <b>else</b> {
        *<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))
    };

    config.override_aud_vals = <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector">vector</a>[];

    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);
}
</code></pre>



</details>

<a id="0x1_keyless_account_add_override_aud_for_next_epoch"></a>

## Function `add_override_aud_for_next_epoch`

Convenience method to queue up an append to the set of override <code>aud</code>'s. The change will only be effective
after reconfiguration. Only callable via governance proposal.

WARNING: If a malicious override <code>aud</code> is set, this *could* lead to stolen funds.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud_for_next_epoch">add_override_aud_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, aud: <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud_for_next_epoch">add_override_aud_for_next_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>, aud: String) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>let</b> config = <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) {
        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()
    } <b>else</b> {
        *<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))
    };

    <a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> config.override_aud_vals, aud);

    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);
}
</code></pre>



</details>

<a id="0x1_keyless_account_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the queued up configuration changes, if there are any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_on_new_epoch">on_new_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_on_new_epoch">on_new_epoch</a>(fx: &<a href="../../../aptos-stdlib/../move-stdlib/tests/compiler-v2-doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>, <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;()) {
        <b>let</b> vk = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>();
        <b>if</b> (<b>exists</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(@aptos_framework)) {
            *<b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(@aptos_framework) = vk;
        } <b>else</b> {
            <b>move_to</b>(fx, vk);
        }
    };

    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) {
        <b>let</b> config = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>();
        <b>if</b> (<b>exists</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(@aptos_framework)) {
            *<b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(@aptos_framework) = config;
        } <b>else</b> {
            <b>move_to</b>(fx, config);
        }
    };
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify=<b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
