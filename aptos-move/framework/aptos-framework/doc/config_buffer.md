
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


<pre><code>use 0x1::any;
use 0x1::option;
use 0x1::simple_map;
use 0x1::string;
use 0x1::system_addresses;
use 0x1::type_info;
</code></pre>



<a id="0x1_config_buffer_PendingConfigs"></a>

## Resource `PendingConfigs`



<pre><code>struct PendingConfigs has key
</code></pre>



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


<pre><code>const ESTD_SIGNER_NEEDED: u64 &#61; 1;
</code></pre>



<a id="0x1_config_buffer_initialize"></a>

## Function `initialize`



<pre><code>public fun initialize(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    if (!exists&lt;PendingConfigs&gt;(@aptos_framework)) &#123;
        move_to(aptos_framework, PendingConfigs &#123;
            configs: simple_map::new(),
        &#125;)
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_config_buffer_does_exist"></a>

## Function `does_exist`

Check whether there is a pending config payload for <code>T</code>.


<pre><code>public fun does_exist&lt;T: store&gt;(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun does_exist&lt;T: store&gt;(): bool acquires PendingConfigs &#123;
    if (exists&lt;PendingConfigs&gt;(@aptos_framework)) &#123;
        let config &#61; borrow_global&lt;PendingConfigs&gt;(@aptos_framework);
        simple_map::contains_key(&amp;config.configs, &amp;type_info::type_name&lt;T&gt;())
    &#125; else &#123;
        false
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_config_buffer_upsert"></a>

## Function `upsert`

Upsert an on-chain config to the buffer for the next epoch.

Typically used in <code>X::set_for_next_epoch()</code> where X is an on-chain config.


<pre><code>public(friend) fun upsert&lt;T: drop, store&gt;(config: T)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun upsert&lt;T: drop &#43; store&gt;(config: T) acquires PendingConfigs &#123;
    let configs &#61; borrow_global_mut&lt;PendingConfigs&gt;(@aptos_framework);
    let key &#61; type_info::type_name&lt;T&gt;();
    let value &#61; any::pack(config);
    simple_map::upsert(&amp;mut configs.configs, key, value);
&#125;
</code></pre>



</details>

<a id="0x1_config_buffer_extract"></a>

## Function `extract`

Take the buffered config <code>T</code> out (buffer cleared). Abort if the buffer is empty.
Should only be used at the end of a reconfiguration.

Typically used in <code>X::on_new_epoch()</code> where X is an on-chaon config.


<pre><code>public fun extract&lt;T: store&gt;(): T
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun extract&lt;T: store&gt;(): T acquires PendingConfigs &#123;
    let configs &#61; borrow_global_mut&lt;PendingConfigs&gt;(@aptos_framework);
    let key &#61; type_info::type_name&lt;T&gt;();
    let (_, value_packed) &#61; simple_map::remove(&amp;mut configs.configs, &amp;key);
    any::unpack(value_packed)
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;
</code></pre>



<a id="@Specification_1_does_exist"></a>

### Function `does_exist`


<pre><code>public fun does_exist&lt;T: store&gt;(): bool
</code></pre>




<pre><code>aborts_if false;
let type_name &#61; type_info::type_name&lt;T&gt;();
ensures result &#61;&#61; spec_fun_does_exist&lt;T&gt;(type_name);
</code></pre>




<a id="0x1_config_buffer_spec_fun_does_exist"></a>


<pre><code>fun spec_fun_does_exist&lt;T: store&gt;(type_name: String): bool &#123;
   if (exists&lt;PendingConfigs&gt;(@aptos_framework)) &#123;
       let config &#61; global&lt;PendingConfigs&gt;(@aptos_framework);
       simple_map::spec_contains_key(config.configs, type_name)
   &#125; else &#123;
       false
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_upsert"></a>

### Function `upsert`


<pre><code>public(friend) fun upsert&lt;T: drop, store&gt;(config: T)
</code></pre>




<pre><code>aborts_if !exists&lt;PendingConfigs&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_extract"></a>

### Function `extract`


<pre><code>public fun extract&lt;T: store&gt;(): T
</code></pre>




<pre><code>aborts_if !exists&lt;PendingConfigs&gt;(@aptos_framework);
include ExtractAbortsIf&lt;T&gt;;
</code></pre>




<a id="0x1_config_buffer_ExtractAbortsIf"></a>


<pre><code>schema ExtractAbortsIf&lt;T&gt; &#123;
    let configs &#61; global&lt;PendingConfigs&gt;(@aptos_framework);
    let key &#61; type_info::type_name&lt;T&gt;();
    aborts_if !simple_map::spec_contains_key(configs.configs, key);
    include any::UnpackAbortsIf&lt;T&gt; &#123;
        x: simple_map::spec_get(configs.configs, key)
    &#125;;
&#125;
</code></pre>




<a id="0x1_config_buffer_SetForNextEpochAbortsIf"></a>


<pre><code>schema SetForNextEpochAbortsIf &#123;
    account: &amp;signer;
    config: vector&lt;u8&gt;;
    let account_addr &#61; std::signer::address_of(account);
    aborts_if account_addr !&#61; @aptos_framework;
    aborts_if len(config) &#61;&#61; 0;
    aborts_if !exists&lt;PendingConfigs&gt;(@aptos_framework);
&#125;
</code></pre>




<a id="0x1_config_buffer_OnNewEpochAbortsIf"></a>


<pre><code>schema OnNewEpochAbortsIf&lt;T&gt; &#123;
    let type_name &#61; type_info::type_name&lt;T&gt;();
    let configs &#61; global&lt;PendingConfigs&gt;(@aptos_framework);
    include spec_fun_does_exist&lt;T&gt;(type_name) &#61;&#61;&gt; any::UnpackAbortsIf&lt;T&gt; &#123;
        x: simple_map::spec_get(configs.configs, type_name)
    &#125;;
&#125;
</code></pre>




<a id="0x1_config_buffer_OnNewEpochRequirement"></a>


<pre><code>schema OnNewEpochRequirement&lt;T&gt; &#123;
    let type_name &#61; type_info::type_name&lt;T&gt;();
    let configs &#61; global&lt;PendingConfigs&gt;(@aptos_framework);
    include spec_fun_does_exist&lt;T&gt;(type_name) &#61;&#61;&gt; any::UnpackRequirement&lt;T&gt; &#123;
        x: simple_map::spec_get(configs.configs, type_name)
    &#125;;
&#125;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
