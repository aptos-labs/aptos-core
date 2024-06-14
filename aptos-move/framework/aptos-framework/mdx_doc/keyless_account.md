
<a id="0x1_keyless_account"></a>

# Module `0x1::keyless_account`

This module is responsible for configuring keyless blockchain accounts which were introduced in
[AIP&#45;61](https://github.com/aptos&#45;foundation/AIPs/blob/main/aips/aip&#45;61.md).


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


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra">0x1::bn254_algebra</a>;<br /><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;<br /><b>use</b> <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519">0x1::ed25519</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_keyless_account_Group"></a>

## Struct `Group`



<pre><code>&#35;[resource_group(&#35;[scope &#61; <b>global</b>])]<br /><b>struct</b> <a href="keyless_account.md#0x1_keyless_account_Group">Group</a><br /></code></pre>



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

The 288&#45;byte Groth16 verification key (VK) for the ZK relation that implements keyless accounts


<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="keyless_account.md#0x1_keyless_account_Group">0x1::keyless_account::Group</a>])]<br /><b>struct</b> <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a> <b>has</b> drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 32&#45;byte serialization of <code>alpha &#42; G</code>, where <code>G</code> is the generator of <code>G1</code>.
</dd>
<dt>
<code>beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64&#45;byte serialization of <code>alpha &#42; H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64&#45;byte serialization of <code>gamma &#42; H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64&#45;byte serialization of <code>delta &#42; H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 <code>\<b>forall</b> i \in &#123;0, ..., \ell&#125;, 64&#45;byte serialization of gamma^&#123;&#45;1&#125; &#42; (beta &#42; a_i &#43; alpha &#42; b_i &#43; c_i) &#42; H</code>, where
 <code>H</code> is the generator of <code>G1</code> and <code>\ell</code> is 1 for the ZK relation.
</dd>
</dl>


</details>

<a id="0x1_keyless_account_Configuration"></a>

## Resource `Configuration`



<pre><code>&#35;[resource_group_member(&#35;[group &#61; <a href="keyless_account.md#0x1_keyless_account_Group">0x1::keyless_account::Group</a>])]<br /><b>struct</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>override_aud_vals: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>
 An override <code>aud</code> for the identity of a recovery service, which will help users recover their keyless accounts
 associated with dapps or wallets that have disappeared.
 IMPORTANT: This recovery service &#42;&#42;cannot&#42;&#42; on its own take over user accounts; a user must first sign in
 via OAuth in the recovery service in order to allow it to rotate any of that user&apos;s keyless accounts.
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
<code>training_wheels_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
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
 The max length of the value of the JWT&apos;s <code>iss</code> field supported in our circuit (e.g., <code>&quot;https://accounts.google.com&quot;</code>)
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
 The max length of the base64url&#45;encoded JWT header in bytes supported in our circuit
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION"></a>

A serialized BN254 G1 point is invalid.


<pre><code><b>const</b> <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION">E_INVALID_BN254_G1_SERIALIZATION</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION"></a>

A serialized BN254 G2 point is invalid.


<pre><code><b>const</b> <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE"></a>

The training wheels PK needs to be 32 bytes long.


<pre><code><b>const</b> <a href="keyless_account.md#0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_keyless_account_new_groth16_verification_key"></a>

## Function `new_groth16_verification_key`



<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />                                        beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />                                        gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />                                        delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,<br />                                        gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;<br />): <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a> &#123;<br />    <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a> &#123;<br />        alpha_g1,<br />        beta_g2,<br />        gamma_g2,<br />        delta_g2,<br />        gamma_abc_g1,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_new_configuration"></a>

## Function `new_configuration`



<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_configuration">new_configuration</a>(override_aud_val: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, max_signatures_per_txn: u16, max_exp_horizon_secs: u64, training_wheels_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, max_commited_epk_bytes: u16, max_iss_val_bytes: u16, max_extra_field_bytes: u16, max_jwt_header_b64_bytes: u32): <a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_new_configuration">new_configuration</a>(<br />    override_aud_val: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    max_signatures_per_txn: u16,<br />    max_exp_horizon_secs: u64,<br />    training_wheels_pubkey: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    max_commited_epk_bytes: u16,<br />    max_iss_val_bytes: u16,<br />    max_extra_field_bytes: u16,<br />    max_jwt_header_b64_bytes: u32<br />): <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />        override_aud_vals: override_aud_val,<br />        max_signatures_per_txn,<br />        max_exp_horizon_secs,<br />        training_wheels_pubkey,<br />        max_commited_epk_bytes,<br />        max_iss_val_bytes,<br />        max_extra_field_bytes,<br />        max_jwt_header_b64_bytes,<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_validate_groth16_vk"></a>

## Function `validate_groth16_vk`

Pre&#45;validate the VK to actively&#45;prevent incorrect VKs from being set on&#45;chain.


<pre><code><b>fun</b> <a href="keyless_account.md#0x1_keyless_account_validate_groth16_vk">validate_groth16_vk</a>(vk: &amp;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="keyless_account.md#0x1_keyless_account_validate_groth16_vk">validate_groth16_vk</a>(vk: &amp;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>) &#123;<br />    // Could be leveraged <b>to</b> speed up the VM deserialization of the VK by 2x, since it can <b>assume</b> the points are valid.<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_G1">bn254_algebra::G1</a>, <a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_FormatG1Compr">bn254_algebra::FormatG1Compr</a>&gt;(&amp;vk.alpha_g1)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION">E_INVALID_BN254_G1_SERIALIZATION</a>);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_G2">bn254_algebra::G2</a>, <a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_FormatG2Compr">bn254_algebra::FormatG2Compr</a>&gt;(&amp;vk.beta_g2)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_G2">bn254_algebra::G2</a>, <a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_FormatG2Compr">bn254_algebra::FormatG2Compr</a>&gt;(&amp;vk.gamma_g2)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_G2">bn254_algebra::G2</a>, <a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_FormatG2Compr">bn254_algebra::FormatG2Compr</a>&gt;(&amp;vk.delta_g2)), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G2_SERIALIZATION">E_INVALID_BN254_G2_SERIALIZATION</a>);<br />    for (i in 0..<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;vk.gamma_abc_g1)) &#123;<br />        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;<a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;<a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_G1">bn254_algebra::G1</a>, <a href="../../aptos-stdlib/doc/bn254_algebra.md#0x1_bn254_algebra_FormatG1Compr">bn254_algebra::FormatG1Compr</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;vk.gamma_abc_g1, i))), <a href="keyless_account.md#0x1_keyless_account_E_INVALID_BN254_G1_SERIALIZATION">E_INVALID_BN254_G1_SERIALIZATION</a>);<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_update_groth16_verification_key"></a>

## Function `update_groth16_verification_key`

Sets the Groth16 verification key, only callable during genesis. To call during governance proposals, use
<code>set_groth16_verification_key_for_next_epoch</code>.

WARNING: See <code>set_groth16_verification_key_for_next_epoch</code> for caveats.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_groth16_verification_key">update_groth16_verification_key</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_groth16_verification_key">update_groth16_verification_key</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br />    // There should not be a previous resource set here.<br />    <b>move_to</b>(fx, vk);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_update_configuration"></a>

## Function `update_configuration`

Sets the keyless configuration, only callable during genesis. To call during governance proposals, use
<code>set_configuration_for_next_epoch</code>.

WARNING: See <code>set_configuration_for_next_epoch</code> for caveats.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_configuration">update_configuration</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_configuration">update_configuration</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br />    // There should not be a previous resource set here.<br />    <b>move_to</b>(fx, config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_update_training_wheels"></a>

## Function `update_training_wheels`



<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels">update_training_wheels</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels">update_training_wheels</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;pk)) &#123;<br />        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;pk)) &#61;&#61; 32, <a href="keyless_account.md#0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>)<br />    &#125;;<br /><br />    <b>let</b> config &#61; <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));<br />    config.training_wheels_pubkey &#61; pk;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_update_max_exp_horizon"></a>

## Function `update_max_exp_horizon`



<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon">update_max_exp_horizon</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon">update_max_exp_horizon</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>let</b> config &#61; <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));<br />    config.max_exp_horizon_secs &#61; max_exp_horizon_secs;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_remove_all_override_auds"></a>

## Function `remove_all_override_auds`



<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds">remove_all_override_auds</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds">remove_all_override_auds</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>let</b> config &#61; <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));<br />    config.override_aud_vals &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_add_override_aud"></a>

## Function `add_override_aud`



<pre><code>&#35;[deprecated]<br /><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud">add_override_aud</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aud: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud">add_override_aud</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aud: String) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>let</b> config &#61; <b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> config.override_aud_vals, aud);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_set_groth16_verification_key_for_next_epoch"></a>

## Function `set_groth16_verification_key_for_next_epoch`

Queues up a change to the Groth16 verification key. The change will only be effective after reconfiguration.
Only callable via governance proposal.

WARNING: To mitigate against DoS attacks, a VK change should be done together with a training wheels PK change,
so that old ZKPs for the old VK cannot be replayed as potentially&#45;valid ZKPs.

WARNING: If a malicious key is set, this would lead to stolen funds.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_groth16_verification_key_for_next_epoch">set_groth16_verification_key_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">keyless_account::Groth16VerificationKey</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_groth16_verification_key_for_next_epoch">set_groth16_verification_key_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(vk);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_set_configuration_for_next_epoch"></a>

## Function `set_configuration_for_next_epoch`

Queues up a change to the keyless configuration. The change will only be effective after reconfiguration. Only
callable via governance proposal.

WARNING: A malicious <code><a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a></code> could lead to DoS attacks, create liveness issues, or enable a malicious
recovery service provider to phish users&apos; accounts.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">keyless_account::Configuration</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_update_training_wheels_for_next_epoch"></a>

## Function `update_training_wheels_for_next_epoch`

Convenience method to queue up a change to the training wheels PK. The change will only be effective after
reconfiguration. Only callable via governance proposal.

WARNING: If a malicious key is set, this &#42;could&#42; lead to stolen funds.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels_for_next_epoch">update_training_wheels_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_training_wheels_for_next_epoch">update_training_wheels_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br /><br />    // If a PK is being set, validate it first.<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;pk)) &#123;<br />        <b>let</b> bytes &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&amp;pk);<br />        <b>let</b> vpk &#61; <a href="../../aptos-stdlib/doc/ed25519.md#0x1_ed25519_new_validated_public_key_from_bytes">ed25519::new_validated_public_key_from_bytes</a>(bytes);<br />        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;vpk), <a href="keyless_account.md#0x1_keyless_account_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>)<br />    &#125;;<br /><br />    <b>let</b> config &#61; <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) &#123;<br />        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()<br />    &#125; <b>else</b> &#123;<br />        &#42;<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))<br />    &#125;;<br /><br />    config.training_wheels_pubkey &#61; pk;<br /><br />    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_update_max_exp_horizon_for_next_epoch"></a>

## Function `update_max_exp_horizon_for_next_epoch`

Convenience method to queues up a change to the max expiration horizon. The change will only be effective after
reconfiguration. Only callable via governance proposal.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon_for_next_epoch">update_max_exp_horizon_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_update_max_exp_horizon_for_next_epoch">update_max_exp_horizon_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_exp_horizon_secs: u64) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br /><br />    <b>let</b> config &#61; <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) &#123;<br />        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()<br />    &#125; <b>else</b> &#123;<br />        &#42;<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))<br />    &#125;;<br /><br />    config.max_exp_horizon_secs &#61; max_exp_horizon_secs;<br /><br />    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_remove_all_override_auds_for_next_epoch"></a>

## Function `remove_all_override_auds_for_next_epoch`

Convenience method to queue up clearing the set of override <code>aud</code>&apos;s. The change will only be effective after
reconfiguration. Only callable via governance proposal.

WARNING: When no override <code>aud</code> is set, recovery of keyless accounts associated with applications that disappeared
is no longer possible.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds_for_next_epoch">remove_all_override_auds_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_remove_all_override_auds_for_next_epoch">remove_all_override_auds_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br /><br />    <b>let</b> config &#61; <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) &#123;<br />        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()<br />    &#125; <b>else</b> &#123;<br />        &#42;<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))<br />    &#125;;<br /><br />    config.override_aud_vals &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];<br /><br />    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_add_override_aud_for_next_epoch"></a>

## Function `add_override_aud_for_next_epoch`

Convenience method to queue up an append to to the set of override <code>aud</code>&apos;s. The change will only be effective
after reconfiguration. Only callable via governance proposal.

WARNING: If a malicious override <code>aud</code> is set, this &#42;could&#42; lead to stolen funds.


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud_for_next_epoch">add_override_aud_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aud: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_add_override_aud_for_next_epoch">add_override_aud_for_next_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aud: String) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br /><br />    <b>let</b> config &#61; <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) &#123;<br />        <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()<br />    &#125; <b>else</b> &#123;<br />        &#42;<b>borrow_global</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))<br />    &#125;;<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> config.override_aud_vals, aud);<br /><br />    <a href="keyless_account.md#0x1_keyless_account_set_configuration_for_next_epoch">set_configuration_for_next_epoch</a>(fx, config);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_keyless_account_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the queued up configuration changes, if there are any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_on_new_epoch">on_new_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="keyless_account.md#0x1_keyless_account_on_new_epoch">on_new_epoch</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>, <a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br /><br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;()) &#123;<br />        <b>let</b> vk &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>();<br />        <b>if</b> (<b>exists</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(@aptos_framework) &#61; vk;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(fx, vk);<br />        &#125;<br />    &#125;;<br /><br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;()) &#123;<br />        <b>let</b> config &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>();<br />        <b>if</b> (<b>exists</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="keyless_account.md#0x1_keyless_account_Configuration">Configuration</a>&gt;(@aptos_framework) &#61; config;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(fx, config);<br />        &#125;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify&#61;<b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
