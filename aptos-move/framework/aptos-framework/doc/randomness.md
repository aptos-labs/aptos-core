
<a id="0x1_randomness"></a>

# Module `0x1::randomness`

On-chain randomness utils.


-  [Resource `PerBlockRandomness`](#0x1_randomness_PerBlockRandomness)
-  [Function `initialize`](#0x1_randomness_initialize)
-  [Function `on_new_block`](#0x1_randomness_on_new_block)
-  [Specification](#@Specification_0)
    -  [Function `initialize`](#@Specification_0_initialize)
    -  [Function `on_new_block`](#@Specification_0_on_new_block)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_randomness_PerBlockRandomness"></a>

## Resource `PerBlockRandomness`

Per-block randomness seed.
This resource is updated in every block prologue.


<pre><code><b>struct</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_randomness_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>move_to</b>(framework, <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
        seed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    });
}
</code></pre>



</details>

<a id="0x1_randomness_on_new_block"></a>

## Function `on_new_block`

Invoked in block prologues to update the block-level randomness seed.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed_for_new_block: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed_for_new_block: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);
    <b>let</b> seed_holder = <b>borrow_global_mut</b>&lt;<a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a>&gt;(@aptos_framework);
    seed_holder.seed = seed_for_new_block;
}
</code></pre>



</details>

<a id="@Specification_0"></a>

## Specification


<a id="@Specification_0_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> framework_addr = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(framework);
<b>aborts_if</b> framework_addr != @aptos_framework;
<b>aborts_if</b> <b>exists</b>&lt;<a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a>&gt;(framework_addr);
<b>ensures</b> <b>global</b>&lt;<a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a>&gt;(framework_addr).seed == <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_none">option::spec_none</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;();
</code></pre>



<a id="@Specification_0_on_new_block"></a>

### Function `on_new_block`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, seed_for_new_block: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>




<pre><code><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(vm) != @vm;
<b>aborts_if</b> !<b>exists</b>&lt;<a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a>&gt;(@aptos_framework);
<b>ensures</b> <b>global</b>&lt;<a href="randomness.md#0x1_randomness_PerBlockRandomness">PerBlockRandomness</a>&gt;(@aptos_framework).seed == seed_for_new_block;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
