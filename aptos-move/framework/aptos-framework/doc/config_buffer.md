
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
-  [Function `extract_v2`](#0x1_config_buffer_extract_v2)
-  [Specification](#@Specification_1)
    -  [Function `does_exist`](#@Specification_1_does_exist)
    -  [Function `upsert`](#@Specification_1_upsert)
    -  [Function `extract_v2`](#@Specification_1_extract_v2)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">0x1::any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
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


<a id="0x1_config_buffer_EDEPRECATED"></a>

Function is deprecated.


<pre><code><b>const</b> <a href="config_buffer.md#0x1_config_buffer_EDEPRECATED">EDEPRECATED</a>: u64 = 2;
</code></pre>



<a id="0x1_config_buffer_ESTD_SIGNER_NEEDED"></a>

Config buffer operations failed with permission denied.


<pre><code><b>const</b> <a href="config_buffer.md#0x1_config_buffer_ESTD_SIGNER_NEEDED">ESTD_SIGNER_NEEDED</a>: u64 = 1;
</code></pre>



<a id="0x1_config_buffer_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(aptos_framework, <a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a> {
            configs: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>(),
        })
    }
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

Use <code>extract_v2</code> instead.


<pre><code>#[deprecated]
<b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_extract">extract</a>&lt;T: store&gt;(): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_extract">extract</a>&lt;T: store&gt;(): T {
    <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_unavailable">error::unavailable</a>(<a href="config_buffer.md#0x1_config_buffer_EDEPRECATED">EDEPRECATED</a>))
}
</code></pre>



</details>

<a id="0x1_config_buffer_extract_v2"></a>

## Function `extract_v2`

Take the buffered config <code>T</code> out (buffer cleared). Abort if the buffer is empty.
Should only be used at the end of a reconfiguration.

Typically used in <code>X::on_new_epoch()</code> where X is an on-chaon config.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_extract_v2">extract_v2</a>&lt;T: store&gt;(): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_extract_v2">extract_v2</a>&lt;T: store&gt;(): T <b>acquires</b> <a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a> {
    <b>let</b> configs = <b>borrow_global_mut</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
    <b>let</b> key = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
    <b>let</b> (_, value_packed) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> configs.configs, &key);
    <a href="../../aptos-stdlib/doc/any.md#0x1_any_unpack">any::unpack</a>(value_packed)
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
</code></pre>



<a id="@Specification_1_does_exist"></a>

### Function `does_exist`


<pre><code><b>public</b> <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_does_exist">does_exist</a>&lt;T: store&gt;(): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
<b>let</b> type_name = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
<b>ensures</b> result == <a href="config_buffer.md#0x1_config_buffer_spec_fun_does_exist">spec_fun_does_exist</a>&lt;T&gt;(type_name);
</code></pre>




<a id="0x1_config_buffer_spec_fun_does_exist"></a>


<pre><code><b>fun</b> <a href="config_buffer.md#0x1_config_buffer_spec_fun_does_exist">spec_fun_does_exist</a>&lt;T: store&gt;(type_name: String): bool {
   <b>if</b> (<b>exists</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework)) {
       <b>let</b> config = <b>global</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
       <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(config.configs, type_name)
   } <b>else</b> {
       <b>false</b>
   }
}
</code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_upsert">upsert</a>&lt;T: drop, store&gt;(config: T)
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_extract_v2"></a>

### Function `extract_v2`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="config_buffer.md#0x1_config_buffer_extract_v2">extract_v2</a>&lt;T: store&gt;(): T
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
<b>include</b> <a href="config_buffer.md#0x1_config_buffer_ExtractAbortsIf">ExtractAbortsIf</a>&lt;T&gt;;
</code></pre>




<a id="0x1_config_buffer_ExtractAbortsIf"></a>


<pre><code><b>schema</b> <a href="config_buffer.md#0x1_config_buffer_ExtractAbortsIf">ExtractAbortsIf</a>&lt;T&gt; {
    <b>let</b> configs = <b>global</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
    <b>let</b> key = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
    <b>aborts_if</b> !<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_contains_key">simple_map::spec_contains_key</a>(configs.configs, key);
    <b>include</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any_UnpackAbortsIf">any::UnpackAbortsIf</a>&lt;T&gt; {
        self: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(configs.configs, key)
    };
}
</code></pre>




<a id="0x1_config_buffer_SetForNextEpochAbortsIf"></a>


<pre><code><b>schema</b> <a href="config_buffer.md#0x1_config_buffer_SetForNextEpochAbortsIf">SetForNextEpochAbortsIf</a> {
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;
    config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
    <b>let</b> account_addr = std::signer::address_of(<a href="account.md#0x1_account">account</a>);
    <b>aborts_if</b> account_addr != @aptos_framework;
    <b>aborts_if</b> len(config) == 0;
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
}
</code></pre>




<a id="0x1_config_buffer_OnNewEpochAbortsIf"></a>


<pre><code><b>schema</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochAbortsIf">OnNewEpochAbortsIf</a>&lt;T&gt; {
    <b>let</b> type_name = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
    <b>let</b> configs = <b>global</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_spec_fun_does_exist">spec_fun_does_exist</a>&lt;T&gt;(type_name) ==&gt; <a href="../../aptos-stdlib/doc/any.md#0x1_any_UnpackAbortsIf">any::UnpackAbortsIf</a>&lt;T&gt; {
        self: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(configs.configs, type_name)
    };
}
</code></pre>




<a id="0x1_config_buffer_OnNewEpochRequirement"></a>


<pre><code><b>schema</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">OnNewEpochRequirement</a>&lt;T&gt; {
    <b>let</b> type_name = <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;T&gt;();
    <b>let</b> configs = <b>global</b>&lt;<a href="config_buffer.md#0x1_config_buffer_PendingConfigs">PendingConfigs</a>&gt;(@aptos_framework);
    <b>include</b> <a href="config_buffer.md#0x1_config_buffer_spec_fun_does_exist">spec_fun_does_exist</a>&lt;T&gt;(type_name) ==&gt; <a href="../../aptos-stdlib/doc/any.md#0x1_any_UnpackRequirement">any::UnpackRequirement</a>&lt;T&gt; {
        self: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_spec_get">simple_map::spec_get</a>(configs.configs, type_name)
    };
}
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
