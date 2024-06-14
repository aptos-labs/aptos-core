
<a id="0x1_randomness_api_v0_config"></a>

# Module `0x1::randomness_api_v0_config`



-  [Resource `RequiredGasDeposit`](#0x1_randomness_api_v0_config_RequiredGasDeposit)
-  [Resource `AllowCustomMaxGasFlag`](#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag)
-  [Function `initialize`](#0x1_randomness_api_v0_config_initialize)
-  [Function `set_for_next_epoch`](#0x1_randomness_api_v0_config_set_for_next_epoch)
-  [Function `set_allow_max_gas_flag_for_next_epoch`](#0x1_randomness_api_v0_config_set_allow_max_gas_flag_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_randomness_api_v0_config_on_new_epoch)
-  [Specification](#@Specification_0)


<pre><code><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /></code></pre>



<a id="0x1_randomness_api_v0_config_RequiredGasDeposit"></a>

## Resource `RequiredGasDeposit`



<pre><code><b>struct</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a> <b>has</b> drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>gas_amount: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_randomness_api_v0_config_AllowCustomMaxGasFlag"></a>

## Resource `AllowCustomMaxGasFlag`

If this flag is set, <code>max_gas</code> specified inside <code>&#35;[<a href="randomness.md#0x1_randomness">randomness</a>()]</code> will be used as the required deposit.


<pre><code><b>struct</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a> <b>has</b> drop, store, key<br /></code></pre>



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


<pre><code><b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_initialize">initialize</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, required_amount: <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">randomness_api_v0_config::RequiredGasDeposit</a>, allow_custom_max_gas_flag: <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">randomness_api_v0_config::AllowCustomMaxGasFlag</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_initialize">initialize</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, required_amount: <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a>, allow_custom_max_gas_flag: <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br />    <b>move_to</b>(framework, required_amount);<br />    <b>move_to</b>(framework, allow_custom_max_gas_flag);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_api_v0_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on&#45;chain governance to update <code><a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a></code> for the next epoch.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_amount: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_set_for_next_epoch">set_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_amount: Option&lt;u64&gt;) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a> &#123; gas_amount &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_api_v0_config_set_allow_max_gas_flag_for_next_epoch"></a>

## Function `set_allow_max_gas_flag_for_next_epoch`

This can be called by on&#45;chain governance to update <code><a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a></code> for the next epoch.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_set_allow_max_gas_flag_for_next_epoch">set_allow_max_gas_flag_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, value: bool)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_set_allow_max_gas_flag_for_next_epoch">set_allow_max_gas_flag_for_next_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, value: bool) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a> &#123; value &#125; );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_randomness_api_v0_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a></code>, if there is any.


<pre><code><b>public</b> <b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a>, <a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a>&gt;()) &#123;<br />        <b>let</b> new_config &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_RequiredGasDeposit">RequiredGasDeposit</a>&gt;(@aptos_framework) &#61; new_config;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, new_config);<br />        &#125;<br />    &#125;;<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a>&gt;()) &#123;<br />        <b>let</b> new_config &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="randomness_api_v0_config.md#0x1_randomness_api_v0_config_AllowCustomMaxGasFlag">AllowCustomMaxGasFlag</a>&gt;(@aptos_framework) &#61; new_config;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, new_config);<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_0"></a>

## Specification



<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
