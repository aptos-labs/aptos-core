
<a id="0x1_jwk_consensus_config"></a>

# Module `0x1::jwk_consensus_config`



-  [Resource `JWKConsensusConfig`](#0x1_jwk_consensus_config_JWKConsensusConfig)
-  [Function `initialize`](#0x1_jwk_consensus_config_initialize)
-  [Function `set_for_next_epoch`](#0x1_jwk_consensus_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_jwk_consensus_config_on_new_epoch)


<pre><code><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_jwk_consensus_config_JWKConsensusConfig"></a>

## Resource `JWKConsensusConfig`



<pre><code><b>struct</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_jwk_consensus_config_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <b>move_to</b>(framework, <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> { bytes })
}
</code></pre>



</details>

<a id="0x1_jwk_consensus_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`



<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>let</b> flag = <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> { bytes };
    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(flag);

}
</code></pre>



</details>

<a id="0x1_jwk_consensus_config_on_new_epoch"></a>

## Function `on_new_epoch`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_on_new_epoch">on_new_epoch</a>() <b>acquires</b> <a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a> {
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;()) {
        <b>let</b> new_config = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;();
        <b>borrow_global_mut</b>&lt;<a href="jwk_consensus_config.md#0x1_jwk_consensus_config_JWKConsensusConfig">JWKConsensusConfig</a>&gt;(@aptos_framework).bytes = new_config.bytes;
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
