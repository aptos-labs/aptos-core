
<a id="0x1_eth_trie"></a>

# Module `0x1::eth_trie`



-  [Function `verify_eth_trie_inclusion_proof`](#0x1_eth_trie_verify_eth_trie_inclusion_proof)
-  [Function `verify_eth_trie_exclusion_proof`](#0x1_eth_trie_verify_eth_trie_exclusion_proof)
-  [Function `native_verify_proof_eth_trie`](#0x1_eth_trie_native_verify_proof_eth_trie)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_eth_trie_verify_eth_trie_inclusion_proof"></a>

## Function `verify_eth_trie_inclusion_proof`

Public wrapper function that calls the native and returns a bool.
Returns true if the inclusion proof is valid i.e. the value exists in the tree
Also returns the value corresponding to the key


<pre><code><b>public</b> <b>fun</b> <a href="eth_trie.md#0x1_eth_trie_verify_eth_trie_inclusion_proof">verify_eth_trie_inclusion_proof</a>(root: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): (bool, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="eth_trie.md#0x1_eth_trie_verify_eth_trie_inclusion_proof">verify_eth_trie_inclusion_proof</a>(
    root: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
): (bool, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>let</b> (proof_is_valid, value) = <a href="eth_trie.md#0x1_eth_trie_native_verify_proof_eth_trie">native_verify_proof_eth_trie</a>(root, key, proof);
    (proof_is_valid && !<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&value), value)
}
</code></pre>



</details>

<a id="0x1_eth_trie_verify_eth_trie_exclusion_proof"></a>

## Function `verify_eth_trie_exclusion_proof`

Public wrapper function that calls the native and returns a bool.
Returns true if the exclusion proof is valid i.e. the value does not exist in the tree


<pre><code><b>public</b> <b>fun</b> <a href="eth_trie.md#0x1_eth_trie_verify_eth_trie_exclusion_proof">verify_eth_trie_exclusion_proof</a>(root: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="eth_trie.md#0x1_eth_trie_verify_eth_trie_exclusion_proof">verify_eth_trie_exclusion_proof</a>(
    root: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
): bool {
    <b>let</b> (proof_is_valid, value) = <a href="eth_trie.md#0x1_eth_trie_native_verify_proof_eth_trie">native_verify_proof_eth_trie</a>(root, key, proof);
    proof_is_valid && <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&value)
}
</code></pre>



</details>

<a id="0x1_eth_trie_native_verify_proof_eth_trie"></a>

## Function `native_verify_proof_eth_trie`



<pre><code><b>public</b> <b>fun</b> <a href="eth_trie.md#0x1_eth_trie_native_verify_proof_eth_trie">native_verify_proof_eth_trie</a>(root: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): (bool, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>public</b> <b>fun</b> <a href="eth_trie.md#0x1_eth_trie_native_verify_proof_eth_trie">native_verify_proof_eth_trie</a>(
    root: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    proof: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
): (bool, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;);
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
