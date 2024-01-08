
<a id="0x1_randomness"></a>

# Module `0x1::randomness`

On-chain randomness utils.


-  [Resource `BlockRandomness`](#0x1_randomness_BlockRandomness)
-  [Function `on_new_block`](#0x1_randomness_on_new_block)


<pre><code><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_randomness_BlockRandomness"></a>

## Resource `BlockRandomness`

The block-level randomness seed.
It's updated by BlockMetadata transaction at the beginning of the execution of every block.


<pre><code><b>struct</b> <a href="randomness.md#0x1_randomness_BlockRandomness">BlockRandomness</a> <b>has</b> drop, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>block_randomness: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_randomness_on_new_block"></a>

## Function `on_new_block`

Invoked in <code>block_prologue_ext()</code> to update the block-level seed randomness.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_on_new_block">on_new_block</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, randomness_available: bool, block_randomness: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="randomness.md#0x1_randomness_on_new_block">on_new_block</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, randomness_available: bool, block_randomness: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="randomness.md#0x1_randomness_BlockRandomness">BlockRandomness</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(<a href="account.md#0x1_account">account</a>);
    <b>if</b> (<b>exists</b>&lt;<a href="randomness.md#0x1_randomness_BlockRandomness">BlockRandomness</a>&gt;(@aptos_framework)) {
        <b>move_from</b>&lt;<a href="randomness.md#0x1_randomness_BlockRandomness">BlockRandomness</a>&gt;(@aptos_framework);
    };
    <b>if</b> (randomness_available) {
        <b>move_to</b>(<a href="account.md#0x1_account">account</a>, <a href="randomness.md#0x1_randomness_BlockRandomness">BlockRandomness</a> { block_randomness })
    };
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
