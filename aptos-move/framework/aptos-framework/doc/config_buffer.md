
<a id="0x1_config_buffer"></a>

# Module `0x1::config_buffer`

This wrapper helps store an on-chain config for the next epoch.

Once reconfigure with DKG is introduced, every on-chain config <code>C</code> should do the following.
- Support async update when DKG is enabled. This is typically done by 3 steps below.
- Implement <code>C::set_for_next_epoch()</code> using <code>upsert()</code> function in this module.
- Implement <code>C::on_new_epoch()</code> using <code>extract()</code> function in this module.
- Update <code>0x1::reconfiguration_with_dkg::finish()</code> to call <code>C::on_new_epoch()</code>.
- Support sychronous update when DKG is disabled.
This is typically done by implementing <code>C::set()</code> to update the config resource directly.

NOTE: on-chain config <code>0x1::state::ValidatorSet</code> implemented its own buffer.


-  [Resource `PendingConfigs`](#0x1_config_buffer_PendingConfigs)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_config_buffer_initialize)
-  [Function `does_exist`](#0x1_config_buffer_does_exist)
-  [Function `upsert`](#0x1_config_buffer_upsert)
-  [Function `extract`](#0x1_config_buffer_extract)
-  [Specification](#@Specification_1)
    -  [Function `does_exist`](#@Specification_1_does_exist)
    -  [Function `upsert`](#@Specification_1_upsert)
    -  [Function `extract`](#@Specification_1_extract)


<pre><code>use 0x1::any;<br/>use 0x1::option;<br/>use 0x1::simple_map;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/>use 0x1::type_info;<br/></code></pre>



<a id="0x1_config_buffer_PendingConfigs"></a>

## Resource `PendingConfigs`



<pre><code>struct PendingConfigs has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>configs: simple_map::SimpleMap&lt;string::String, any::Any&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_config_buffer_ESTD_SIGNER_NEEDED"></a>

Config buffer operations failed with permission denied.


<pre><code>const ESTD_SIGNER_NEEDED: u64 &#61; 1;<br/></code></pre>



<a id="0x1_config_buffer_initialize"></a>

## Function `initialize`



<pre><code>public fun initialize(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    if (!exists&lt;PendingConfigs&gt;(@aptos_framework)) &#123;<br/>        move_to(aptos_framework, PendingConfigs &#123;<br/>            configs: simple_map::new(),<br/>        &#125;)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_config_buffer_does_exist"></a>

## Function `does_exist`

Check whether there is a pending config payload for <code>T</code>.


<pre><code>public fun does_exist&lt;T: store&gt;(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun does_exist&lt;T: store&gt;(): bool acquires PendingConfigs &#123;<br/>    if (exists&lt;PendingConfigs&gt;(@aptos_framework)) &#123;<br/>        let config &#61; borrow_global&lt;PendingConfigs&gt;(@aptos_framework);<br/>        simple_map::contains_key(&amp;config.configs, &amp;type_info::type_name&lt;T&gt;())<br/>    &#125; else &#123;<br/>        false<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_config_buffer_upsert"></a>

## Function `upsert`

Upsert an on-chain config to the buffer for the next epoch.

Typically used in <code>X::set_for_next_epoch()</code> where X is an on-chain config.


<pre><code>public(friend) fun upsert&lt;T: drop, store&gt;(config: T)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun upsert&lt;T: drop &#43; store&gt;(config: T) acquires PendingConfigs &#123;<br/>    let configs &#61; borrow_global_mut&lt;PendingConfigs&gt;(@aptos_framework);<br/>    let key &#61; type_info::type_name&lt;T&gt;();<br/>    let value &#61; any::pack(config);<br/>    simple_map::upsert(&amp;mut configs.configs, key, value);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_config_buffer_extract"></a>

## Function `extract`

Take the buffered config <code>T</code> out (buffer cleared). Abort if the buffer is empty.
Should only be used at the end of a reconfiguration.

Typically used in <code>X::on_new_epoch()</code> where X is an on-chaon config.


<pre><code>public fun extract&lt;T: store&gt;(): T<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract&lt;T: store&gt;(): T acquires PendingConfigs &#123;<br/>    let configs &#61; borrow_global_mut&lt;PendingConfigs&gt;(@aptos_framework);<br/>    let key &#61; type_info::type_name&lt;T&gt;();<br/>    let (_, value_packed) &#61; simple_map::remove(&amp;mut configs.configs, &amp;key);<br/>    any::unpack(value_packed)<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;<br/></code></pre>



<a id="@Specification_1_does_exist"></a>

### Function `does_exist`


<pre><code>public fun does_exist&lt;T: store&gt;(): bool<br/></code></pre>




<pre><code>aborts_if false;<br/>let type_name &#61; type_info::type_name&lt;T&gt;();<br/>ensures result &#61;&#61; spec_fun_does_exist&lt;T&gt;(type_name);<br/></code></pre>




<a id="0x1_config_buffer_spec_fun_does_exist"></a>


<pre><code>fun spec_fun_does_exist&lt;T: store&gt;(type_name: String): bool &#123;<br/>   if (exists&lt;PendingConfigs&gt;(@aptos_framework)) &#123;<br/>       let config &#61; global&lt;PendingConfigs&gt;(@aptos_framework);<br/>       simple_map::spec_contains_key(config.configs, type_name)<br/>   &#125; else &#123;<br/>       false<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code>public(friend) fun upsert&lt;T: drop, store&gt;(config: T)<br/></code></pre>




<pre><code>aborts_if !exists&lt;PendingConfigs&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_extract"></a>

### Function `extract`


<pre><code>public fun extract&lt;T: store&gt;(): T<br/></code></pre>




<pre><code>aborts_if !exists&lt;PendingConfigs&gt;(@aptos_framework);<br/>include ExtractAbortsIf&lt;T&gt;;<br/></code></pre>




<a id="0x1_config_buffer_ExtractAbortsIf"></a>


<pre><code>schema ExtractAbortsIf&lt;T&gt; &#123;<br/>let configs &#61; global&lt;PendingConfigs&gt;(@aptos_framework);<br/>let key &#61; type_info::type_name&lt;T&gt;();<br/>aborts_if !simple_map::spec_contains_key(configs.configs, key);<br/>include any::UnpackAbortsIf&lt;T&gt; &#123;<br/>    x: simple_map::spec_get(configs.configs, key)<br/>&#125;;<br/>&#125;<br/></code></pre>




<a id="0x1_config_buffer_SetForNextEpochAbortsIf"></a>


<pre><code>schema SetForNextEpochAbortsIf &#123;<br/>account: &amp;signer;<br/>config: vector&lt;u8&gt;;<br/>let account_addr &#61; std::signer::address_of(account);<br/>aborts_if account_addr !&#61; @aptos_framework;<br/>aborts_if len(config) &#61;&#61; 0;<br/>aborts_if !exists&lt;PendingConfigs&gt;(@aptos_framework);<br/>&#125;<br/></code></pre>




<a id="0x1_config_buffer_OnNewEpochAbortsIf"></a>


<pre><code>schema OnNewEpochAbortsIf&lt;T&gt; &#123;<br/>let type_name &#61; type_info::type_name&lt;T&gt;();<br/>let configs &#61; global&lt;PendingConfigs&gt;(@aptos_framework);<br/>include spec_fun_does_exist&lt;T&gt;(type_name) &#61;&#61;&gt; any::UnpackAbortsIf&lt;T&gt; &#123;<br/>    x: simple_map::spec_get(configs.configs, type_name)<br/>&#125;;<br/>&#125;<br/></code></pre>




<a id="0x1_config_buffer_OnNewEpochRequirement"></a>


<pre><code>schema OnNewEpochRequirement&lt;T&gt; &#123;<br/>let type_name &#61; type_info::type_name&lt;T&gt;();<br/>let configs &#61; global&lt;PendingConfigs&gt;(@aptos_framework);<br/>include spec_fun_does_exist&lt;T&gt;(type_name) &#61;&#61;&gt; any::UnpackRequirement&lt;T&gt; &#123;<br/>    x: simple_map::spec_get(configs.configs, type_name)<br/>&#125;;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
