
<a id="0x1_evm_genesis_config"></a>

# Module `0x1::evm_genesis_config`



-  [Resource `EvmGenesisConfig`](#0x1_evm_genesis_config_EvmGenesisConfig)
-  [Struct `EvmGenesisEvent`](#0x1_evm_genesis_config_EvmGenesisEvent)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_evm_genesis_config_initialize)
-  [Function `set_for_next_epoch`](#0x1_evm_genesis_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_evm_genesis_config_on_new_epoch)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `set_for_next_epoch`](#@Specification_1_set_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)


<pre><code><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_evm_genesis_config_EvmGenesisConfig"></a>

## Resource `EvmGenesisConfig`

The struct stores the on-chain EVM genesis configuration.


<pre><code><b>struct</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a> <b>has</b> drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_evm_genesis_config_EvmGenesisEvent"></a>

## Struct `EvmGenesisEvent`

Event to signal EVM genesis config has been initialized or updated.


<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisEvent">EvmGenesisEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_evm_genesis_config_EINVALID_CONFIG"></a>

The provided on chain config bytes are empty or invalid


<pre><code><b>const</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_EINVALID_CONFIG">EINVALID_CONFIG</a>: u64 = 1;
</code></pre>



<a id="0x1_evm_genesis_config_initialize"></a>

## Function `initialize`

Publishes the EvmGenesisConfig config.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_initialize">initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_initialize">initialize</a>(
    supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(supra_framework);
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&config), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="evm_genesis_config.md#0x1_evm_genesis_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));
    <b>move_to</b>(supra_framework, <a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a> { config });
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisEvent">EvmGenesisEvent</a> {});
}
</code></pre>



</details>

<a id="0x1_evm_genesis_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on-chain governance to update on-chain evm configs for the next epoch.
Example usage:
```
supra_framework::evm_genesis_config::set_for_next_epoch(&framework_signer, some_config_bytes);
supra_framework::supra_governance::reconfigure(&framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(<a href="account.md#0x1_account">account</a>);
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&config), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="evm_genesis_config.md#0x1_evm_genesis_config_EINVALID_CONFIG">EINVALID_CONFIG</a>));
    std::config_buffer::upsert&lt;<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a>&gt;(<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a> { config });
}
</code></pre>



</details>

<a id="0x1_evm_genesis_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a></code> in buffer, if there is any.
If supra_framework has a EvmGenesisConfig, then update the new config to supra_framework.
Otherwise, move the new config to supra_framework.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_supra_framework">system_addresses::assert_supra_framework</a>(framework);
    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a>&gt;()) {
        <b>let</b> new_config = <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a>&gt;();
        <b>if</b> (!<b>exists</b>&lt;<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a>&gt;(@supra_framework)) {
            <b>move_to</b>(framework, new_config);
            <a href="event.md#0x1_event_emit">event::emit</a>(<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisEvent">EvmGenesisEvent</a> {});
        };
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_initialize">initialize</a>(supra_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>pragma</b> aborts_if_is_strict = <b>false</b>;
</code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_set_for_next_epoch">set_for_next_epoch</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<pre><code><b>include</b> <a href="config_buffer.md#0x1_config_buffer_SetForNextEpochAbortsIf">config_buffer::SetForNextEpochAbortsIf</a>;
</code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="evm_genesis_config.md#0x1_evm_genesis_config_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>requires</b> @supra_framework == std::signer::address_of(framework);
<b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="evm_genesis_config.md#0x1_evm_genesis_config_EvmGenesisConfig">EvmGenesisConfig</a>&gt;;
<b>aborts_if</b> <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
