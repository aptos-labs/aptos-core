
<a id="0x1_randomness_api_v0_config"></a>

# Module `0x1::randomness_api_v0_config`



-  [Resource `RequiredGasDeposit`](#0x1_randomness_api_v0_config_RequiredGasDeposit)
-  [Resource `AllowCustomMaxGasFlag`](#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag)
-  [Function `initialize`](#0x1_randomness_api_v0_config_initialize)
-  [Function `set_for_next_epoch`](#0x1_randomness_api_v0_config_set_for_next_epoch)
-  [Function `set_allow_max_gas_flag_for_next_epoch`](#0x1_randomness_api_v0_config_set_allow_max_gas_flag_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_randomness_api_v0_config_on_new_epoch)
-  [Specification](#@Specification_0)


<pre><code>use 0x1::chain_status;<br/>use 0x1::config_buffer;<br/>use 0x1::option;<br/>use 0x1::system_addresses;<br/></code></pre>



<a id="0x1_randomness_api_v0_config_RequiredGasDeposit"></a>

## Resource `RequiredGasDeposit`



<pre><code>struct RequiredGasDeposit has drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gas_amount: option::Option&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_randomness_api_v0_config_AllowCustomMaxGasFlag"></a>

## Resource `AllowCustomMaxGasFlag`

If this flag is set, <code>max_gas</code> specified inside <code>&#35;[randomness()]</code> will be used as the required deposit.


<pre><code>struct AllowCustomMaxGasFlag has drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>value: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_randomness_api_v0_config_initialize"></a>

## Function `initialize`

Only used in genesis.


<pre><code>fun initialize(framework: &amp;signer, required_amount: randomness_api_v0_config::RequiredGasDeposit, allow_custom_max_gas_flag: randomness_api_v0_config::AllowCustomMaxGasFlag)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize(framework: &amp;signer, required_amount: RequiredGasDeposit, allow_custom_max_gas_flag: AllowCustomMaxGasFlag) &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    chain_status::assert_genesis();<br/>    move_to(framework, required_amount);<br/>    move_to(framework, allow_custom_max_gas_flag);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_randomness_api_v0_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on&#45;chain governance to update <code>RequiredGasDeposit</code> for the next epoch.


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, gas_amount: option::Option&lt;u64&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(framework: &amp;signer, gas_amount: Option&lt;u64&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    config_buffer::upsert(RequiredGasDeposit &#123; gas_amount &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_randomness_api_v0_config_set_allow_max_gas_flag_for_next_epoch"></a>

## Function `set_allow_max_gas_flag_for_next_epoch`

This can be called by on&#45;chain governance to update <code>AllowCustomMaxGasFlag</code> for the next epoch.


<pre><code>public fun set_allow_max_gas_flag_for_next_epoch(framework: &amp;signer, value: bool)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_allow_max_gas_flag_for_next_epoch(framework: &amp;signer, value: bool) &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    config_buffer::upsert(AllowCustomMaxGasFlag &#123; value &#125; );<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_randomness_api_v0_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>RequiredGasDeposit</code>, if there is any.


<pre><code>public fun on_new_epoch(framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun on_new_epoch(framework: &amp;signer) acquires RequiredGasDeposit, AllowCustomMaxGasFlag &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    if (config_buffer::does_exist&lt;RequiredGasDeposit&gt;()) &#123;<br/>        let new_config &#61; config_buffer::extract&lt;RequiredGasDeposit&gt;();<br/>        if (exists&lt;RequiredGasDeposit&gt;(@aptos_framework)) &#123;<br/>            &#42;borrow_global_mut&lt;RequiredGasDeposit&gt;(@aptos_framework) &#61; new_config;<br/>        &#125; else &#123;<br/>            move_to(framework, new_config);<br/>        &#125;<br/>    &#125;;<br/>    if (config_buffer::does_exist&lt;AllowCustomMaxGasFlag&gt;()) &#123;<br/>        let new_config &#61; config_buffer::extract&lt;AllowCustomMaxGasFlag&gt;();<br/>        if (exists&lt;AllowCustomMaxGasFlag&gt;(@aptos_framework)) &#123;<br/>            &#42;borrow_global_mut&lt;AllowCustomMaxGasFlag&gt;(@aptos_framework) &#61; new_config;<br/>        &#125; else &#123;<br/>            move_to(framework, new_config);<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code>pragma verify &#61; false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
