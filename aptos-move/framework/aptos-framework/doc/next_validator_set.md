
<a id="0x1_next_validator_set"></a>

# Module `0x1::next_validator_set`



-  [Resource `NextValidatorSet`](#0x1_next_validator_set_NextValidatorSet)
-  [Function `initialize`](#0x1_next_validator_set_initialize)
-  [Function `save`](#0x1_next_validator_set_save)
-  [Function `clear`](#0x1_next_validator_set_clear)
-  [Function `load`](#0x1_next_validator_set_load)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;
</code></pre>



<a id="0x1_next_validator_set_NextValidatorSet"></a>

## Resource `NextValidatorSet`



<pre><code><b>struct</b> <a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="next_validator_set.md#0x1_next_validator_set">next_validator_set</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_next_validator_set_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>if</b> (!<b>exists</b>&lt;<a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(framework, <a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a> { <a href="next_validator_set.md#0x1_next_validator_set">next_validator_set</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>() } )
    }
}
</code></pre>



</details>

<a id="0x1_next_validator_set_save"></a>

## Function `save`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_save">save</a>(infos: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_save">save</a>(infos: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt;) <b>acquires</b> <a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a> {
    <b>borrow_global_mut</b>&lt;<a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a>&gt;(@aptos_framework).<a href="next_validator_set.md#0x1_next_validator_set">next_validator_set</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(infos);
}
</code></pre>



</details>

<a id="0x1_next_validator_set_clear"></a>

## Function `clear`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_clear">clear</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_clear">clear</a>() <b>acquires</b> <a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a> {
    <b>borrow_global_mut</b>&lt;<a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a>&gt;(@aptos_framework).<a href="next_validator_set.md#0x1_next_validator_set">next_validator_set</a> = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
}
</code></pre>



</details>

<a id="0x1_next_validator_set_load"></a>

## Function `load`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_load">load</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="next_validator_set.md#0x1_next_validator_set_load">load</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt; <b>acquires</b> <a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a> {
    <b>let</b> maybe_set = <b>borrow_global</b>&lt;<a href="next_validator_set.md#0x1_next_validator_set_NextValidatorSet">NextValidatorSet</a>&gt;(@aptos_framework).<a href="next_validator_set.md#0x1_next_validator_set">next_validator_set</a>;
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> maybe_set)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
