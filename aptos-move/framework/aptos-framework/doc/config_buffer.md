
<a id="0x1_config_buffer"></a>

# Module `0x1::config_buffer`

This wrapper helps store an on-chain config for the next epoch.

Once reconfigure with DKG is introduced, every on-chain config <code>C</code> should do the following.
- Support async update when DKG is enabled. This is typically done by 3 steps below.
- Implement <code>C::set_for_next_epoch()</code> using <code><a href="config_buffer.md#0x1_config_buffer_upsert">upsert</a>()</code> function in this module.
- Implement <code>C::on_new_epoch()</code> using <code><a href="config_buffer.md#0x1_config_buffer_extract">extract</a>()</code> function in this module.
- Update <code><a href="reconfiguration_with_dkg.md#0x1_reconfiguration_with_dkg_finish">0x1::reconfiguration_with_dkg::finish</a>()</code> to call <code>C::on_new_epoch()</code>.
- Support sychronous update when DKG is disabled.
This is typically done by implementing <code>C::set()</code> to update the config resource directly.

NOTE: on-chain config <code>0x1::state::ValidatorSet</code> implemented its own buffer.


-  [Resource `PendingConfigs`](#0x1_config_buffer_PendingConfigs)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_config_buffer_initialize)
-  [Function `does_exist`](#0x1_config_buffer_does_exist)
-  [Function `upsert`](#0x1_config_buffer_upsert)
-  [Function `extract`](#0x1_config_buffer_extract)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">0x1::any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info">0x1::type_info</a>;
</code></pre>



<a id="0x1_config_buffer_PendingConfigs"></a>

## Resource `PendingConfigs`



<pre><code><b>struct</b> <a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>configs: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>, <a href="../../aptos-stdlib/doc/any.md#0x1_any_Any">any::Any</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_config_buffer_ESTD_SIGNER_NEEDED"></a>

Config buffer operations failed with permission denied.


<pre><code><b>const</b> <a href="config_buffer.md#0x1_config_buffer_ESTD_SIGNER_NEEDED">ESTD_SIGNER_NEEDED</a>: u64 = 1;
</code></pre>



<a id="0x1_config_buffer_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(aptos_framework, <a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a> {
        configs: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>(),
    })
}
</code></pre>



</details>

<a id="0x1_config_buffer_does_exist"></a>

## Function `does_exist`

Check whether there is a pending config payload for <code>T</code>.


<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_does_exist">does_exist</a>&lt;T: store&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_does_exist">does_exist</a>&lt;T: store&gt;(): bool <b>acquires</b> <a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework)) {
        <b>let</b> config = <b>borrow_global</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&config.configs, &<a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;())
    } <b>else</b> {
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_config_buffer_upsert"></a>

## Function `upsert`

Upsert an on-chain config to the buffer for the next epoch.

Typically used in <code>X::set_for_next_epoch()</code> where X is an on-chain config.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_upsert">upsert</a>&lt;T: drop, store&gt;(config: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_upsert">upsert</a>&lt;T: drop + store&gt;(config: T) <b>acquires</b> <a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a> {
    <b>let</b> configs = <b>borrow_global_mut</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
    <b>let</b> key = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
    <b>let</b> value = <a href="../../aptos-stdlib/doc/any.md#0x1_any_pack">any::pack</a>(config);
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&<b>mut</b> configs.configs, key, value);
}
</code></pre>



</details>

<a id="0x1_config_buffer_extract"></a>

## Function `extract`

Take the buffered config <code>T</code> out (buffer cleared). Abort if the buffer is empty.
Should only be used at the end of a reconfiguration.

Typically used in <code>X::on_new_epoch()</code> where X is an on-chaon config.


<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_extract">extract</a>&lt;T: store&gt;(): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_extract">extract</a>&lt;T: store&gt;(): T <b>acquires</b> <a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a> {
    <b>let</b> configs = <b>borrow_global_mut</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
    <b>let</b> key = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
    <b>let</b> (_, value_packed) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> configs.configs, &key);
    <a href="../../aptos-stdlib/doc/any.md#0x1_any_unpack">any::unpack</a>(value_packed)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
