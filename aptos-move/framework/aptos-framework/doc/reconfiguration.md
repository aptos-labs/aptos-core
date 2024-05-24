
<a id="0x1_reconfiguration"></a>

# Module `0x1::reconfiguration`

Publishes configuration information for validators, and issues reconfiguration events
to synchronize configuration changes for the validators.


-  [Struct `NewEpochEvent`](#0x1_reconfiguration_NewEpochEvent)
-  [Struct `NewEpoch`](#0x1_reconfiguration_NewEpoch)
-  [Resource `Configuration`](#0x1_reconfiguration_Configuration)
-  [Resource `DisableReconfiguration`](#0x1_reconfiguration_DisableReconfiguration)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_reconfiguration_initialize)
-  [Function `disable_reconfiguration`](#0x1_reconfiguration_disable_reconfiguration)
-  [Function `enable_reconfiguration`](#0x1_reconfiguration_enable_reconfiguration)
-  [Function `reconfiguration_enabled`](#0x1_reconfiguration_reconfiguration_enabled)
-  [Function `reconfigure`](#0x1_reconfiguration_reconfigure)
-  [Function `last_reconfiguration_time`](#0x1_reconfiguration_last_reconfiguration_time)
-  [Function `current_epoch`](#0x1_reconfiguration_current_epoch)
-  [Function `emit_genesis_reconfiguration_event`](#0x1_reconfiguration_emit_genesis_reconfiguration_event)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `disable_reconfiguration`](#@Specification_1_disable_reconfiguration)
    -  [Function `enable_reconfiguration`](#@Specification_1_enable_reconfiguration)
    -  [Function `reconfiguration_enabled`](#@Specification_1_reconfiguration_enabled)
    -  [Function `reconfigure`](#@Specification_1_reconfigure)
    -  [Function `last_reconfiguration_time`](#@Specification_1_last_reconfiguration_time)
    -  [Function `current_epoch`](#@Specification_1_current_epoch)
    -  [Function `emit_genesis_reconfiguration_event`](#@Specification_1_emit_genesis_reconfiguration_event)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;<br /><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state">0x1::reconfiguration_state</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;<br /><b>use</b> <a href="storage_gas.md#0x1_storage_gas">0x1::storage_gas</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="transaction_fee.md#0x1_transaction_fee">0x1::transaction_fee</a>;<br /></code></pre>



<a id="0x1_reconfiguration_NewEpochEvent"></a>

## Struct `NewEpochEvent`

Event that signals consensus to start a new epoch,
with new configuration information. This is also called a
&quot;reconfiguration event&quot;


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_reconfiguration_NewEpoch"></a>

## Struct `NewEpoch`

Event that signals consensus to start a new epoch,
with new configuration information. This is also called a
&quot;reconfiguration event&quot;


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="reconfiguration.md#0x1_reconfiguration_NewEpoch">NewEpoch</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_reconfiguration_Configuration"></a>

## Resource `Configuration`

Holds information about state of reconfiguration


<pre><code><b>struct</b> <a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>
 Epoch number
</dd>
<dt>
<code>last_reconfiguration_time: u64</code>
</dt>
<dd>
 Time of last reconfiguration. Only changes on reconfiguration events.
</dd>
<dt>
<code>events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">reconfiguration::NewEpochEvent</a>&gt;</code>
</dt>
<dd>
 Event handle for reconfiguration events
</dd>
</dl>


</details>

<a id="0x1_reconfiguration_DisableReconfiguration"></a>

## Resource `DisableReconfiguration`

Reconfiguration will be disabled if this resource is published under the
aptos_framework system address


<pre><code><b>struct</b> <a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a> <b>has</b> key<br /></code></pre>



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


<a id="0x1_reconfiguration_ECONFIG"></a>

A <code>Reconfiguration</code> resource is in an invalid state


<pre><code><b>const</b> <a href="reconfiguration.md#0x1_reconfiguration_ECONFIG">ECONFIG</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_reconfiguration_ECONFIGURATION"></a>

The <code><a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a></code> resource is in an invalid state


<pre><code><b>const</b> <a href="reconfiguration.md#0x1_reconfiguration_ECONFIGURATION">ECONFIGURATION</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_reconfiguration_EINVALID_BLOCK_TIME"></a>

An invalid block time was encountered.


<pre><code><b>const</b> <a href="reconfiguration.md#0x1_reconfiguration_EINVALID_BLOCK_TIME">EINVALID_BLOCK_TIME</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_reconfiguration_EINVALID_GUID_FOR_EVENT"></a>

An invalid block time was encountered.


<pre><code><b>const</b> <a href="reconfiguration.md#0x1_reconfiguration_EINVALID_GUID_FOR_EVENT">EINVALID_GUID_FOR_EVENT</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_reconfiguration_EMODIFY_CAPABILITY"></a>

A <code>ModifyConfigCapability</code> is in a different state than was expected


<pre><code><b>const</b> <a href="reconfiguration.md#0x1_reconfiguration_EMODIFY_CAPABILITY">EMODIFY_CAPABILITY</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_reconfiguration_initialize"></a>

## Function `initialize`

Only called during genesis.
Publishes <code><a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a></code> resource. Can only be invoked by aptos framework account, and only a single time in Genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    // <b>assert</b> it matches `new_epoch_event_key()`, otherwise the <a href="event.md#0x1_event">event</a> can&apos;t be recognized<br />    <b>assert</b>!(<a href="account.md#0x1_account_get_guid_next_creation_num">account::get_guid_next_creation_num</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)) &#61;&#61; 2, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration.md#0x1_reconfiguration_EINVALID_GUID_FOR_EVENT">EINVALID_GUID_FOR_EVENT</a>));<br />    <b>move_to</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(<br />        aptos_framework,<br />        <a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a> &#123;<br />            epoch: 0,<br />            last_reconfiguration_time: 0,<br />            events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a>&gt;(aptos_framework),<br />        &#125;<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_disable_reconfiguration"></a>

## Function `disable_reconfiguration`

Private function to temporarily halt reconfiguration.
This function should only be used for offline WriteSet generation purpose and should never be invoked on chain.


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_disable_reconfiguration">disable_reconfiguration</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_disable_reconfiguration">disable_reconfiguration</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(<a href="reconfiguration.md#0x1_reconfiguration_reconfiguration_enabled">reconfiguration_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration.md#0x1_reconfiguration_ECONFIGURATION">ECONFIGURATION</a>));<br />    <b>move_to</b>(aptos_framework, <a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a> &#123;&#125;)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_enable_reconfiguration"></a>

## Function `enable_reconfiguration`

Private function to resume reconfiguration.
This function should only be used for offline WriteSet generation purpose and should never be invoked on chain.


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_enable_reconfiguration">enable_reconfiguration</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_enable_reconfiguration">enable_reconfiguration</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br /><br />    <b>assert</b>!(!<a href="reconfiguration.md#0x1_reconfiguration_reconfiguration_enabled">reconfiguration_enabled</a>(), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration.md#0x1_reconfiguration_ECONFIGURATION">ECONFIGURATION</a>));<br />    <a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a> &#123;&#125; &#61; <b>move_from</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_reconfiguration_enabled"></a>

## Function `reconfiguration_enabled`



<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_reconfiguration_enabled">reconfiguration_enabled</a>(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_reconfiguration_enabled">reconfiguration_enabled</a>(): bool &#123;<br />    !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a>&gt;(@aptos_framework)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_reconfigure"></a>

## Function `reconfigure`

Signal validators to start using new configuration. Must be called from friend config modules.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfigure</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfigure</a>() <b>acquires</b> <a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a> &#123;<br />    // Do not do anything <b>if</b> <a href="genesis.md#0x1_genesis">genesis</a> <b>has</b> not finished.<br />    <b>if</b> (<a href="chain_status.md#0x1_chain_status_is_genesis">chain_status::is_genesis</a>() &#124;&#124; <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() &#61;&#61; 0 &#124;&#124; !<a href="reconfiguration.md#0x1_reconfiguration_reconfiguration_enabled">reconfiguration_enabled</a>()) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    <b>let</b> config_ref &#61; <b>borrow_global_mut</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br />    <b>let</b> current_time &#61; <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();<br /><br />    // Do not do anything <b>if</b> a <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> <a href="event.md#0x1_event">event</a> is already emitted within this transaction.<br />    //<br />    // This is OK because:<br />    // &#45; The time changes in every non&#45;empty <a href="block.md#0x1_block">block</a><br />    // &#45; A <a href="block.md#0x1_block">block</a> automatically ends after a transaction that <b>emits</b> a <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> <a href="event.md#0x1_event">event</a>, which is guaranteed by<br />    //   VM <b>spec</b> that all transactions comming after a <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> transaction will be returned <b>as</b> Retry<br />    //   status.<br />    // &#45; Each transaction must emit at most one <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> <a href="event.md#0x1_event">event</a><br />    //<br />    // Thus, this check <b>ensures</b> that a transaction that does multiple &quot;<a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> required&quot; actions <b>emits</b> only<br />    // one <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> <a href="event.md#0x1_event">event</a>.<br />    //<br />    <b>if</b> (current_time &#61;&#61; config_ref.last_reconfiguration_time) &#123;<br />        <b>return</b><br />    &#125;;<br /><br />    <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">reconfiguration_state::on_reconfig_start</a>();<br /><br />    // Reconfiguration &quot;forces the <a href="block.md#0x1_block">block</a>&quot; <b>to</b> end, <b>as</b> mentioned above. Therefore, we must process the collected fees<br />    // explicitly so that staking can distribute them.<br />    //<br />    // This also handles the case when a validator is removed due <b>to</b> the governance proposal. In particular, removing<br />    // the validator causes a <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a>. We explicitly process fees, i.e. we drain aggregatable <a href="coin.md#0x1_coin">coin</a> and populate<br />    // the fees <a href="../../aptos-stdlib/doc/table.md#0x1_table">table</a>, prior <b>to</b> calling `on_new_epoch()`. That call, in turn, distributes transaction fees for all active<br />    // and pending_inactive validators, which <b>include</b> <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> validator that is <b>to</b> be removed.<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_collect_and_distribute_gas_fees">features::collect_and_distribute_gas_fees</a>()) &#123;<br />        // All transactions after <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> are Retry. Therefore, when the next<br />        // <a href="block.md#0x1_block">block</a> starts and tries <b>to</b> assign/burn collected fees it will be just 0 and<br />        // nothing will be assigned.<br />        <a href="transaction_fee.md#0x1_transaction_fee_process_collected_fees">transaction_fee::process_collected_fees</a>();<br />    &#125;;<br /><br />    // Call <a href="stake.md#0x1_stake">stake</a> <b>to</b> compute the new validator set and distribute rewards and transaction fees.<br />    <a href="stake.md#0x1_stake_on_new_epoch">stake::on_new_epoch</a>();<br />    <a href="storage_gas.md#0x1_storage_gas_on_reconfig">storage_gas::on_reconfig</a>();<br /><br />    <b>assert</b>!(current_time &gt; config_ref.last_reconfiguration_time, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration.md#0x1_reconfiguration_EINVALID_BLOCK_TIME">EINVALID_BLOCK_TIME</a>));<br />    config_ref.last_reconfiguration_time &#61; current_time;<br />    <b>spec</b> &#123;<br />        <b>assume</b> config_ref.epoch &#43; 1 &lt;&#61; MAX_U64;<br />    &#125;;<br />    config_ref.epoch &#61; config_ref.epoch &#43; 1;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="reconfiguration.md#0x1_reconfiguration_NewEpoch">NewEpoch</a> &#123;<br />                epoch: config_ref.epoch,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a>&gt;(<br />        &amp;<b>mut</b> config_ref.events,<br />        <a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a> &#123;<br />            epoch: config_ref.epoch,<br />        &#125;,<br />    );<br /><br />    <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_finish">reconfiguration_state::on_reconfig_finish</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_last_reconfiguration_time"></a>

## Function `last_reconfiguration_time`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">last_reconfiguration_time</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">last_reconfiguration_time</a>(): u64 <b>acquires</b> <a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).last_reconfiguration_time<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_current_epoch"></a>

## Function `current_epoch`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">current_epoch</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">current_epoch</a>(): u64 <b>acquires</b> <a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a> &#123;<br />    <b>borrow_global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).epoch<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_emit_genesis_reconfiguration_event"></a>

## Function `emit_genesis_reconfiguration_event`

Emit a <code><a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a></code> event. This function will be invoked by genesis directly to generate the very first
reconfiguration event.


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_emit_genesis_reconfiguration_event">emit_genesis_reconfiguration_event</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_emit_genesis_reconfiguration_event">emit_genesis_reconfiguration_event</a>() <b>acquires</b> <a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a> &#123;<br />    <b>let</b> config_ref &#61; <b>borrow_global_mut</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br />    <b>assert</b>!(config_ref.epoch &#61;&#61; 0 &amp;&amp; config_ref.last_reconfiguration_time &#61;&#61; 0, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration.md#0x1_reconfiguration_ECONFIGURATION">ECONFIGURATION</a>));<br />    config_ref.epoch &#61; 1;<br /><br />    <b>if</b> (std::features::module_event_migration_enabled()) &#123;<br />        <a href="event.md#0x1_event_emit">event::emit</a>(<br />            <a href="reconfiguration.md#0x1_reconfiguration_NewEpoch">NewEpoch</a> &#123;<br />                epoch: config_ref.epoch,<br />            &#125;,<br />        );<br />    &#125;;<br />    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a>&gt;(<br />        &amp;<b>mut</b> config_ref.events,<br />        <a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a> &#123;<br />            epoch: config_ref.epoch,<br />        &#125;,<br />    );<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>The Configuration resource is stored under the Aptos framework account with initial values upon module&apos;s initialization.</td>
<td>Medium</td>
<td>The Configuration resource may only be initialized with specific values and published under the aptos_framework account.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>The reconfiguration status may be determined at any time without causing an abort, indicating whether or not the system allows reconfiguration.</td>
<td>Low</td>
<td>The reconfiguration_enabled function will never abort and always returns a boolean value that accurately represents whether the system allows reconfiguration.</td>
<td>Formally verified via <a href="#high-level-req-2">reconfiguration_enabled</a>.</td>
</tr>

<tr>
<td>3</td>
<td>For each reconfiguration, the epoch value (config_ref.epoch) increases by 1, and one &apos;NewEpochEvent&apos; is emitted.</td>
<td>Critical</td>
<td>After reconfiguration, the reconfigure() function increases the epoch value of the configuration by one and increments the counter of the NewEpochEvent&apos;s EventHandle by one.</td>
<td>Audited that these two values remain in sync.</td>
</tr>

<tr>
<td>4</td>
<td>Reconfiguration is possible only if genesis has started and reconfiguration is enabled. Also, the last reconfiguration must not be the current time, returning early without further actions otherwise.</td>
<td>High</td>
<td>The reconfigure() function may only execute to perform successful reconfiguration when genesis has started and when reconfiguration is enabled. Without satisfying both conditions, the function returns early without executing any further actions.</td>
<td>Formally verified via <a href="#high-level-req-4">reconfigure</a>.</td>
</tr>

<tr>
<td>5</td>
<td>Consecutive reconfigurations without the passage of time are not permitted.</td>
<td>High</td>
<td>The reconfigure() function enforces the restriction that reconfiguration may only be performed when the current time is not equal to the last_reconfiguration_time.</td>
<td>Formally verified via <a href="#high-level-req-5">reconfigure</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br /><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt;<br />    (<a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &gt;&#61; <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">last_reconfiguration_time</a>());<br /></code></pre>


Make sure the signer address is @aptos_framework.


<a id="0x1_reconfiguration_AbortsIfNotAptosFramework"></a>


<pre><code><b>schema</b> <a href="reconfiguration.md#0x1_reconfiguration_AbortsIfNotAptosFramework">AbortsIfNotAptosFramework</a> &#123;<br />aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);<br />&#125;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


Address @aptos_framework must exist resource Account and Configuration.
Already exists in framework account.
Guid_creation_num should be 2 according to logic.


<pre><code><b>include</b> <a href="reconfiguration.md#0x1_reconfiguration_AbortsIfNotAptosFramework">AbortsIfNotAptosFramework</a>;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>let</b> <b>post</b> config &#61; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;Account&gt;(addr);<br /><b>aborts_if</b> !(<b>global</b>&lt;Account&gt;(addr).guid_creation_num &#61;&#61; 2);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>ensures</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br /><b>ensures</b> config.epoch &#61;&#61; 0 &amp;&amp; config.last_reconfiguration_time &#61;&#61; 0;<br /><b>ensures</b> config.events &#61;&#61; <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="reconfiguration.md#0x1_reconfiguration_NewEpochEvent">NewEpochEvent</a>&gt; &#123;<br />    counter: 0,<br />    <a href="guid.md#0x1_guid">guid</a>: <a href="guid.md#0x1_guid_GUID">guid::GUID</a> &#123;<br />        id: <a href="guid.md#0x1_guid_ID">guid::ID</a> &#123;<br />            creation_num: 2,<br />            addr: @aptos_framework<br />        &#125;<br />    &#125;<br />&#125;;<br /></code></pre>



<a id="@Specification_1_disable_reconfiguration"></a>

### Function `disable_reconfiguration`


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_disable_reconfiguration">disable_reconfiguration</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="reconfiguration.md#0x1_reconfiguration_AbortsIfNotAptosFramework">AbortsIfNotAptosFramework</a>;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_enable_reconfiguration"></a>

### Function `enable_reconfiguration`


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_enable_reconfiguration">enable_reconfiguration</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>


Make sure the caller is admin and check the resource DisableReconfiguration.


<pre><code><b>include</b> <a href="reconfiguration.md#0x1_reconfiguration_AbortsIfNotAptosFramework">AbortsIfNotAptosFramework</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a>&gt;(@aptos_framework);<br /><b>ensures</b> !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_reconfiguration_enabled"></a>

### Function `reconfiguration_enabled`


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_reconfiguration_enabled">reconfiguration_enabled</a>(): bool<br /></code></pre>




<pre><code>// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> result &#61;&#61; !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_DisableReconfiguration">DisableReconfiguration</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_reconfigure"></a>

### Function `reconfigure`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfigure</a>()<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> verify_duration_estimate &#61; 600;<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>let</b> success &#61; !(<a href="chain_status.md#0x1_chain_status_is_genesis">chain_status::is_genesis</a>() &#124;&#124; <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() &#61;&#61; 0 &#124;&#124; !<a href="reconfiguration.md#0x1_reconfiguration_reconfiguration_enabled">reconfiguration_enabled</a>())<br />    &amp;&amp; <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>() !&#61; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).last_reconfiguration_time;<br /><b>include</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_spec_periodical_reward_rate_decrease_enabled">features::spec_periodical_reward_rate_decrease_enabled</a>() &#61;&#61;&gt; <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigEnabledRequirement">staking_config::StakingRewardsConfigEnabledRequirement</a>;<br /><b>include</b> success &#61;&#61;&gt; <a href="aptos_coin.md#0x1_aptos_coin_ExistsAptosCoin">aptos_coin::ExistsAptosCoin</a>;<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>aborts_if</b> <b>false</b>;<br /><b>ensures</b> success &#61;&#61;&gt; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).epoch &#61;&#61; <b>old</b>(<b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).epoch) &#43; 1;<br /><b>ensures</b> success &#61;&#61;&gt; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).last_reconfiguration_time &#61;&#61; <a href="timestamp.md#0x1_timestamp_spec_now_microseconds">timestamp::spec_now_microseconds</a>();<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a> and <a id="high-level-req-5" href="#high-level-req">high&#45;level requirement 5</a>:
<b>ensures</b> !success &#61;&#61;&gt; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).epoch &#61;&#61; <b>old</b>(<b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).epoch);<br /></code></pre>



<a id="@Specification_1_last_reconfiguration_time"></a>

### Function `last_reconfiguration_time`


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_last_reconfiguration_time">last_reconfiguration_time</a>(): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br /><b>ensures</b> result &#61;&#61; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).last_reconfiguration_time;<br /></code></pre>



<a id="@Specification_1_current_epoch"></a>

### Function `current_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">current_epoch</a>(): u64<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br /><b>ensures</b> result &#61;&#61; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).epoch;<br /></code></pre>



<a id="@Specification_1_emit_genesis_reconfiguration_event"></a>

### Function `emit_genesis_reconfiguration_event`


<pre><code><b>fun</b> <a href="reconfiguration.md#0x1_reconfiguration_emit_genesis_reconfiguration_event">emit_genesis_reconfiguration_event</a>()<br /></code></pre>


When genesis_event emit the epoch and the <code>last_reconfiguration_time</code> .
Should equal to 0


<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br /><b>let</b> config_ref &#61; <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !(config_ref.epoch &#61;&#61; 0 &amp;&amp; config_ref.last_reconfiguration_time &#61;&#61; 0);<br /><b>ensures</b> <b>global</b>&lt;<a href="reconfiguration.md#0x1_reconfiguration_Configuration">Configuration</a>&gt;(@aptos_framework).epoch &#61;&#61; 1;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
