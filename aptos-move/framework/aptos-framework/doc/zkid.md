
<a id="0x1_zkid"></a>

# Module `0x1::zkid`



-  [Struct `Group`](#0x1_zkid_Group)
-  [Resource `Groth16VerificationKey`](#0x1_zkid_Groth16VerificationKey)
-  [Resource `Configuration`](#0x1_zkid_Configuration)
-  [Constants](#@Constants_0)
-  [Function `new_groth16_verification_key`](#0x1_zkid_new_groth16_verification_key)
-  [Function `new_configuration`](#0x1_zkid_new_configuration)
-  [Function `update_groth16_verification_key`](#0x1_zkid_update_groth16_verification_key)
-  [Function `update_configuration`](#0x1_zkid_update_configuration)
-  [Function `update_training_wheels`](#0x1_zkid_update_training_wheels)
-  [Function `remove_all_override_auds`](#0x1_zkid_remove_all_override_auds)
-  [Function `add_override_aud`](#0x1_zkid_add_override_aud)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_zkid_Group"></a>

## Struct `Group`



<pre><code>#[resource_group(#[scope = <b>global</b>])]
<b>struct</b> <a href="zkid.md#0x1_zkid_Group">Group</a>
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

<a id="0x1_zkid_Groth16VerificationKey"></a>

## Resource `Groth16VerificationKey`

The 288-byte Groth16 verification key (VK) for the zkID relation.


<pre><code>#[resource_group_member(#[group = <a href="zkid.md#0x1_zkid_Group">0x1::zkid::Group</a>])]
<b>struct</b> <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 32-byte serialization of <code>alpha * G</code>, where <code>G</code> is the generator of <code>G1</code>.
</dd>
<dt>
<code>beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>alpha * H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>gamma * H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 64-byte serialization of <code>delta * H</code>, where <code>H</code> is the generator of <code>G2</code>.
</dd>
<dt>
<code>gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 <code>\<b>forall</b> i \in {0, ..., \ell}, 64-byte serialization of gamma^{-1} * (beta * a_i + alpha * b_i + c_i) * H</code>, where
 <code>H</code> is the generator of <code>G1</code> and <code>\ell</code> is 1 for the zkID relation.
</dd>
</dl>


</details>

<a id="0x1_zkid_Configuration"></a>

## Resource `Configuration`



<pre><code>#[resource_group_member(#[group = <a href="zkid.md#0x1_zkid_Group">0x1::zkid::Group</a>])]
<b>struct</b> <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>override_aud_vals: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;</code>
</dt>
<dd>
 An override <code>aud</code> for the identity of a recovery service, which will help users recover their zkID accounts
 associated with dapps or wallets that have disappeared.
 IMPORTANT: This recovery service **cannot** on its own take over user accounts; a user must first sign in
 via OAuth in the recovery service in order to allow it to rotate any of that user's zkID accounts.
</dd>
<dt>
<code>max_zkid_signatures_per_txn: u16</code>
</dt>
<dd>
 No transaction can have more than this many zkID signatures.
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
<code>nonce_commitment_num_bytes: u16</code>
</dt>
<dd>
 The size of the "nonce commitment (to the EPK and expiration date)" stored in the JWT's <code>nonce</code> field.
</dd>
<dt>
<code>max_commited_epk_bytes: u16</code>
</dt>
<dd>
 The max length of an ephemeral public key supported in our circuit (93 bytes)
</dd>
<dt>
<code>max_iss_field_bytes: u16</code>
</dt>
<dd>
 The max length of the field name and value of the JWT's <code>iss</code> field supported in our circuit (e.g., <code>"iss":"aptos.com"</code>)
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


<a id="0x1_zkid_E_TRAINING_WHEELS_PK_WRONG_SIZE"></a>

The training wheels PK needs to be 32 bytes long.


<pre><code><b>const</b> <a href="zkid.md#0x1_zkid_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>: u64 = 1;
</code></pre>



<a id="0x1_zkid_new_groth16_verification_key"></a>

## Function `new_groth16_verification_key`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="zkid.md#0x1_zkid_Groth16VerificationKey">zkid::Groth16VerificationKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
                                        gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
): <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
    <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    }
}
</code></pre>



</details>

<a id="0x1_zkid_new_configuration"></a>

## Function `new_configuration`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_new_configuration">new_configuration</a>(override_aud_val: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, max_zkid_signatures_per_txn: u16, max_exp_horizon_secs: u64, training_wheels_pubkey: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, nonce_commitment_num_bytes: u16, max_commited_epk_bytes: u16, max_iss_field_bytes: u16, max_extra_field_bytes: u16, max_jwt_header_b64_bytes: u32): <a href="zkid.md#0x1_zkid_Configuration">zkid::Configuration</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_new_configuration">new_configuration</a>(
    override_aud_val: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,
    max_zkid_signatures_per_txn: u16,
    max_exp_horizon_secs: u64,
    training_wheels_pubkey: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    nonce_commitment_num_bytes: u16,
    max_commited_epk_bytes: u16,
    max_iss_field_bytes: u16,
    max_extra_field_bytes: u16,
    max_jwt_header_b64_bytes: u32
): <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> {
    <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> {
        override_aud_vals: override_aud_val,
        max_zkid_signatures_per_txn,
        max_exp_horizon_secs,
        training_wheels_pubkey,
        nonce_commitment_num_bytes,
        max_commited_epk_bytes,
        max_iss_field_bytes,
        max_extra_field_bytes,
        max_jwt_header_b64_bytes,
    }
}
</code></pre>



</details>

<a id="0x1_zkid_update_groth16_verification_key"></a>

## Function `update_groth16_verification_key`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_update_groth16_verification_key">update_groth16_verification_key</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="zkid.md#0x1_zkid_Groth16VerificationKey">zkid::Groth16VerificationKey</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_update_groth16_verification_key">update_groth16_verification_key</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a>) <b>acquires</b> <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>if</b> (<b>exists</b>&lt;<a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))) {
        <b>let</b> <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
            alpha_g1: _,
            beta_g2: _,
            gamma_g2: _,
            delta_g2: _,
            gamma_abc_g1: _
        } = <b>move_from</b>&lt;<a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    };

    <b>move_to</b>(fx, vk);
}
</code></pre>



</details>

<a id="0x1_zkid_update_configuration"></a>

## Function `update_configuration`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_update_configuration">update_configuration</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="zkid.md#0x1_zkid_Configuration">zkid::Configuration</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_update_configuration">update_configuration</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="zkid.md#0x1_zkid_Configuration">Configuration</a>) <b>acquires</b> <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>if</b> (<b>exists</b>&lt;<a href="zkid.md#0x1_zkid_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx))) {
        <b>let</b> <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> {
            override_aud_vals: _,
            max_zkid_signatures_per_txn: _,
            max_exp_horizon_secs: _,
            training_wheels_pubkey: _,
            nonce_commitment_num_bytes: _,
            max_commited_epk_bytes: _,
            max_iss_field_bytes: _,
            max_extra_field_bytes: _,
            max_jwt_header_b64_bytes: _,
        } = <b>move_from</b>&lt;<a href="zkid.md#0x1_zkid_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    };

    <b>move_to</b>(fx, config);
}
</code></pre>



</details>

<a id="0x1_zkid_update_training_wheels"></a>

## Function `update_training_wheels`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_update_training_wheels">update_training_wheels</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_update_training_wheels">update_training_wheels</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pk: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&pk)) {
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&pk)) == 32, <a href="zkid.md#0x1_zkid_E_TRAINING_WHEELS_PK_WRONG_SIZE">E_TRAINING_WHEELS_PK_WRONG_SIZE</a>)
    };

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="zkid.md#0x1_zkid_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    config.training_wheels_pubkey = pk;
}
</code></pre>



</details>

<a id="0x1_zkid_remove_all_override_auds"></a>

## Function `remove_all_override_auds`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_remove_all_override_auds">remove_all_override_auds</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_remove_all_override_auds">remove_all_override_auds</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="zkid.md#0x1_zkid_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    config.override_aud_vals = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
}
</code></pre>



</details>

<a id="0x1_zkid_add_override_aud"></a>

## Function `add_override_aud`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_add_override_aud">add_override_aud</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aud: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_add_override_aud">add_override_aud</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, aud: String) <b>acquires</b> <a href="zkid.md#0x1_zkid_Configuration">Configuration</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="zkid.md#0x1_zkid_Configuration">Configuration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> config.override_aud_vals, aud);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
