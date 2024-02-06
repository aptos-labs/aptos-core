
<a id="0x1_zkid"></a>

# Module `0x1::zkid`



-  [Struct `ConfigGroup`](#0x1_zkid_ConfigGroup)
-  [Resource `Groth16VerificationKey`](#0x1_zkid_Groth16VerificationKey)
-  [Resource `Configs`](#0x1_zkid_Configs)
-  [Function `initialize`](#0x1_zkid_initialize)
-  [Function `new_groth16_verification_key`](#0x1_zkid_new_groth16_verification_key)
-  [Function `devnet_groth16_vk`](#0x1_zkid_devnet_groth16_vk)
-  [Function `devnet_constants`](#0x1_zkid_devnet_constants)
-  [Function `set_groth16_verification_key`](#0x1_zkid_set_groth16_verification_key)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_zkid_ConfigGroup"></a>

## Struct `ConfigGroup`



<pre><code>#[resource_group(#[scope = <b>global</b>])]
<b>struct</b> <a href="zkid.md#0x1_zkid_ConfigGroup">ConfigGroup</a>
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


<pre><code>#[resource_group_member(#[group = <a href="zkid.md#0x1_zkid_ConfigGroup">0x1::zkid::ConfigGroup</a>])]
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
 64-byte serialization of <code>\<b>forall</b> i \in {0, 1}, gamma^{-1} * (beta * a_i + alpha * b_i + c_i) * H</code>, where <code>H</code> is the generator of <code>G1</code>.
</dd>
</dl>


</details>

<a id="0x1_zkid_Configs"></a>

## Resource `Configs`



<pre><code>#[resource_group_member(#[group = <a href="zkid.md#0x1_zkid_ConfigGroup">0x1::zkid::ConfigGroup</a>])]
<b>struct</b> <a href="zkid.md#0x1_zkid_Configs">Configs</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>max_zkid_signatures_per_txn: u16</code>
</dt>
<dd>

</dd>
<dt>
<code>max_exp_horizon: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_zkid_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="zkid.md#0x1_zkid_initialize">initialize</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="zkid.md#0x1_zkid_Groth16VerificationKey">zkid::Groth16VerificationKey</a>, constants: <a href="zkid.md#0x1_zkid_Configs">zkid::Configs</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="zkid.md#0x1_zkid_initialize">initialize</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, vk: <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a>, constants: <a href="zkid.md#0x1_zkid_Configs">Configs</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);

    <b>move_to</b>(fx, vk);
    <b>move_to</b>(fx, constants);
}
</code></pre>



</details>

<a id="0x1_zkid_new_groth16_verification_key"></a>

## Function `new_groth16_verification_key`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="zkid.md#0x1_zkid_Groth16VerificationKey">zkid::Groth16VerificationKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
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

<a id="0x1_zkid_devnet_groth16_vk"></a>

## Function `devnet_groth16_vk`

Returns the Groth16 VK for our devnet deployment.


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_devnet_groth16_vk">devnet_groth16_vk</a>(): <a href="zkid.md#0x1_zkid_Groth16VerificationKey">zkid::Groth16VerificationKey</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_devnet_groth16_vk">devnet_groth16_vk</a>(): <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
    <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
        alpha_g1: x"6d1c152d2705e35fe7a07a66eb8a10a7f42f1e38c412fbbc3ac7f9affc25dc24",
        beta_g2: x"e20a834c55ae6e2fcbd66636e09322727f317aff8957dd342afa11f936ef7c02cfdc8c9862849a0442bcaa4e03f45343e8bf261ef4ab58cead2efc17100a3b16",
        gamma_g2: x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19",
        delta_g2: x"98c9283068e4bfc51266dcbabffb56bebeb65ece8d9104609026d0d89187961d0c69a4688b23f8a813ee74349785d116aedfcf3f3de15d7c9123b32eba326f23",
        gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
            x"29f65be8be6b13c84c1b29d219f35b998db14be4f7506fff4a475512ef0d959f",
            x"1ddc291dfd35684b634f03cda96ae18139db1653471921c555b2750cbf49908c",
        ],
    }
}
</code></pre>



</details>

<a id="0x1_zkid_devnet_constants"></a>

## Function `devnet_constants`



<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_devnet_constants">devnet_constants</a>(): <a href="zkid.md#0x1_zkid_Configs">zkid::Configs</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="zkid.md#0x1_zkid_devnet_constants">devnet_constants</a>(): <a href="zkid.md#0x1_zkid_Configs">Configs</a> {
    // TODO(<a href="zkid.md#0x1_zkid">zkid</a>): Put reasonable defaults here.
    <a href="zkid.md#0x1_zkid_Configs">Configs</a> {
        max_zkid_signatures_per_txn: 3,
        max_exp_horizon: 100_255_944, // 1159.55 days
    }
}
</code></pre>



</details>

<a id="0x1_zkid_set_groth16_verification_key"></a>

## Function `set_groth16_verification_key`



<pre><code><b>public</b> entry <b>fun</b> <a href="zkid.md#0x1_zkid_set_groth16_verification_key">set_groth16_verification_key</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="zkid.md#0x1_zkid_set_groth16_verification_key">set_groth16_verification_key</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, alpha_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, beta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, delta_g2: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, gamma_abc_g1: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="zkid.md#0x1_zkid_Groth16VerificationKey">Groth16VerificationKey</a> {
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

    <b>let</b> vk = <a href="zkid.md#0x1_zkid_new_groth16_verification_key">new_groth16_verification_key</a>(alpha_g1, beta_g2, gamma_g2, delta_g2, gamma_abc_g1);
    <b>move_to</b>(fx, vk);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
