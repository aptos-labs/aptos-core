
<a id="0x1_gas_schedule"></a>

# Module `0x1::gas_schedule`

This module defines structs and methods to initialize the gas schedule, which dictates how much
it costs to execute Move on the network.


-  [Struct `GasEntry`](#0x1_gas_schedule_GasEntry)
-  [Resource `GasSchedule`](#0x1_gas_schedule_GasSchedule)
-  [Resource `GasScheduleV2`](#0x1_gas_schedule_GasScheduleV2)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_gas_schedule_initialize)
-  [Function `set_gas_schedule`](#0x1_gas_schedule_set_gas_schedule)
-  [Function `set_for_next_epoch`](#0x1_gas_schedule_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_gas_schedule_on_new_epoch)
-  [Function `set_storage_gas_config`](#0x1_gas_schedule_set_storage_gas_config)
-  [Function `set_storage_gas_config_for_next_epoch`](#0x1_gas_schedule_set_storage_gas_config_for_next_epoch)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `set_gas_schedule`](#@Specification_1_set_gas_schedule)
    -  [Function `set_for_next_epoch`](#@Specification_1_set_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)
    -  [Function `set_storage_gas_config`](#@Specification_1_set_storage_gas_config)
    -  [Function `set_storage_gas_config_for_next_epoch`](#@Specification_1_set_storage_gas_config_for_next_epoch)


<pre><code><b>use</b> <a href="chain_status.md#0x1_chain_status">0x1::chain_status</a>;<br /><b>use</b> <a href="config_buffer.md#0x1_config_buffer">0x1::config_buffer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;<br /><b>use</b> <a href="storage_gas.md#0x1_storage_gas">0x1::storage_gas</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="util.md#0x1_util">0x1::util</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_gas_schedule_GasEntry"></a>

## Struct `GasEntry`



<pre><code><b>struct</b> <a href="gas_schedule.md#0x1_gas_schedule_GasEntry">GasEntry</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
<dt>
<code>val: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_gas_schedule_GasSchedule"></a>

## Resource `GasSchedule`



<pre><code><b>struct</b> <a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a> <b>has</b> <b>copy</b>, drop, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasEntry">gas_schedule::GasEntry</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_gas_schedule_GasScheduleV2"></a>

## Resource `GasScheduleV2`



<pre><code><b>struct</b> <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> <b>has</b> <b>copy</b>, drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>feature_version: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>entries: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasEntry">gas_schedule::GasEntry</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION"></a>



<pre><code><b>const</b> <a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION">EINVALID_GAS_FEATURE_VERSION</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_gas_schedule_EINVALID_GAS_SCHEDULE"></a>

The provided gas schedule bytes are empty or invalid


<pre><code><b>const</b> <a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_SCHEDULE">EINVALID_GAS_SCHEDULE</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_gas_schedule_initialize"></a>

## Function `initialize`

Only called during genesis.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;gas_schedule_blob), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_SCHEDULE">EINVALID_GAS_SCHEDULE</a>));<br /><br />    // TODO(Gas): check <b>if</b> gas schedule is consistent<br />    <b>let</b> <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>: <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> &#61; from_bytes(gas_schedule_blob);<br />    <b>move_to</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(aptos_framework, <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_gas_schedule_set_gas_schedule"></a>

## Function `set_gas_schedule`

Deprecated by <code><a href="gas_schedule.md#0x1_gas_schedule_set_for_next_epoch">set_for_next_epoch</a>()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_gas_schedule">set_gas_schedule</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_gas_schedule">set_gas_schedule</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a>, <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;gas_schedule_blob), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_SCHEDULE">EINVALID_GAS_SCHEDULE</a>));<br />    <a href="chain_status.md#0x1_chain_status_assert_genesis">chain_status::assert_genesis</a>();<br /><br />    <b>if</b> (<b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a> &#61; <b>borrow_global_mut</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework);<br />        <b>let</b> new_gas_schedule: <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> &#61; from_bytes(gas_schedule_blob);<br />        <b>assert</b>!(new_gas_schedule.feature_version &gt;&#61; <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>.feature_version,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION">EINVALID_GAS_FEATURE_VERSION</a>));<br />        // TODO(Gas): check <b>if</b> gas schedule is consistent<br />        &#42;<a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a> &#61; new_gas_schedule;<br />    &#125;<br />    <b>else</b> &#123;<br />        <b>if</b> (<b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a>&gt;(@aptos_framework)) &#123;<br />            _ &#61; <b>move_from</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasSchedule">GasSchedule</a>&gt;(@aptos_framework);<br />        &#125;;<br />        <b>let</b> new_gas_schedule: <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> &#61; from_bytes(gas_schedule_blob);<br />        // TODO(Gas): check <b>if</b> gas schedule is consistent<br />        <b>move_to</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(aptos_framework, new_gas_schedule);<br />    &#125;;<br /><br />    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so validator nodes can sync on the updated gas schedule.<br />    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_gas_schedule_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

Set the gas schedule for the next epoch, typically called by on&#45;chain governance.
Abort if the version of the given schedule is lower than the current version.

Example usage:
```
aptos_framework::gas_schedule::set_for_next_epoch(&amp;framework_signer, some_gas_schedule_blob);
aptos_framework::aptos_governance::reconfigure(&amp;framework_signer);
```


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_for_next_epoch">set_for_next_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_for_next_epoch">set_for_next_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&amp;gas_schedule_blob), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_SCHEDULE">EINVALID_GAS_SCHEDULE</a>));<br />    <b>let</b> new_gas_schedule: <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> &#61; from_bytes(gas_schedule_blob);<br />    <b>if</b> (<b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> cur_gas_schedule &#61; <b>borrow_global</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework);<br />        <b>assert</b>!(<br />            new_gas_schedule.feature_version &gt;&#61; cur_gas_schedule.feature_version,<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="gas_schedule.md#0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION">EINVALID_GAS_FEATURE_VERSION</a>)<br />        );<br />    &#125;;<br />    <a href="config_buffer.md#0x1_config_buffer_upsert">config_buffer::upsert</a>(new_gas_schedule);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_gas_schedule_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code><a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a></code>, if there is any.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);<br />    <b>if</b> (<a href="config_buffer.md#0x1_config_buffer_does_exist">config_buffer::does_exist</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;()) &#123;<br />        <b>let</b> new_gas_schedule &#61; <a href="config_buffer.md#0x1_config_buffer_extract">config_buffer::extract</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;();<br />        <b>if</b> (<b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework)) &#123;<br />            &#42;<b>borrow_global_mut</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework) &#61; new_gas_schedule;<br />        &#125; <b>else</b> &#123;<br />            <b>move_to</b>(framework, new_gas_schedule);<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_gas_schedule_set_storage_gas_config"></a>

## Function `set_storage_gas_config`



<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config">set_storage_gas_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config">set_storage_gas_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: StorageGasConfig) &#123;<br />    <a href="storage_gas.md#0x1_storage_gas_set_config">storage_gas::set_config</a>(aptos_framework, config);<br />    // Need <b>to</b> trigger <a href="reconfiguration.md#0x1_reconfiguration">reconfiguration</a> so the VM is guaranteed <b>to</b> load the new gas fee starting from the next<br />    // transaction.<br />    <a href="reconfiguration.md#0x1_reconfiguration_reconfigure">reconfiguration::reconfigure</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_gas_schedule_set_storage_gas_config_for_next_epoch"></a>

## Function `set_storage_gas_config_for_next_epoch`



<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config_for_next_epoch">set_storage_gas_config_for_next_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config_for_next_epoch">set_storage_gas_config_for_next_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: StorageGasConfig) &#123;<br />    <a href="storage_gas.md#0x1_storage_gas_set_config">storage_gas::set_config</a>(aptos_framework, config);<br />&#125;<br /></code></pre>



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
<td>During genesis, the Aptos framework account should be assigned the gas schedule resource.</td>
<td>Medium</td>
<td>The gas_schedule::initialize function calls the assert_aptos_framework function to ensure that the signer is the aptos_framework and then assigns the GasScheduleV2 resource to it.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Only the Aptos framework account should be allowed to update the gas schedule resource.</td>
<td>Critical</td>
<td>The gas_schedule::set_gas_schedule function calls the assert_aptos_framework function to ensure that the signer is the aptos framework account.</td>
<td>Formally verified via <a href="#high-level-req-2">set_gas_schedule</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Only valid gas schedule should be allowed for initialization and update.</td>
<td>Medium</td>
<td>The initialize and set_gas_schedule functions ensures that the gas_schedule_blob is not empty.</td>
<td>Formally verified via <a href="#high-level-req-3.3">initialize</a> and <a href="#high-level-req-3.2">set_gas_schedule</a>.</td>
</tr>

<tr>
<td>4</td>
<td>Only a gas schedule with the feature version greater or equal than the current feature version is allowed to be provided when performing an update operation.</td>
<td>Medium</td>
<td>The set_gas_schedule function validates the feature_version of the new_gas_schedule by ensuring that it is greater or equal than the current gas_schedule.feature_version.</td>
<td>Formally verified via <a href="#high-level-req-4">set_gas_schedule</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br />// This enforces <a id="high-level-req-1" href="#high-level-req">high&#45;level requirement 1</a>:
<b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br />// This enforces <a id="high-level-req-3.3" href="#high-level-req">high&#45;level requirement 3</a>:
<b>aborts_if</b> len(gas_schedule_blob) &#61;&#61; 0;<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(addr);<br /></code></pre>



<a id="@Specification_1_set_gas_schedule"></a>

### Function `set_gas_schedule`


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_gas_schedule">set_gas_schedule</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 600;<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br /><b>requires</b> <a href="chain_status.md#0x1_chain_status_is_genesis">chain_status::is_genesis</a>();<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br />// This enforces <a id="high-level-req-2" href="#high-level-req">high&#45;level requirement 2</a>:
<b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br />// This enforces <a id="high-level-req-3.2" href="#high-level-req">high&#45;level requirement 3</a>:
<b>aborts_if</b> len(gas_schedule_blob) &#61;&#61; 0;<br /><b>let</b> new_gas_schedule &#61; <a href="util.md#0x1_util_spec_from_bytes">util::spec_from_bytes</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(gas_schedule_blob);<br /><b>let</b> <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a> &#61; <b>global</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework);<br />// This enforces <a id="high-level-req-4" href="#high-level-req">high&#45;level requirement 4</a>:
<b>aborts_if</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework) &amp;&amp; new_gas_schedule.feature_version &lt; <a href="gas_schedule.md#0x1_gas_schedule">gas_schedule</a>.feature_version;<br /><b>ensures</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework));<br /><b>ensures</b> <b>global</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework) &#61;&#61; new_gas_schedule;<br /></code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_for_next_epoch">set_for_next_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, gas_schedule_blob: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_SetForNextEpochAbortsIf">config_buffer::SetForNextEpochAbortsIf</a> &#123;<br />    <a href="account.md#0x1_account">account</a>: aptos_framework,<br />    config: gas_schedule_blob<br />&#125;;<br /><b>let</b> new_gas_schedule &#61; <a href="util.md#0x1_util_spec_from_bytes">util::spec_from_bytes</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(gas_schedule_blob);<br /><b>let</b> cur_gas_schedule &#61; <b>global</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> <b>exists</b>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;(@aptos_framework) &amp;&amp; new_gas_schedule.feature_version &lt; cur_gas_schedule.feature_version;<br /></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_on_new_epoch">on_new_epoch</a>(framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>requires</b> @aptos_framework &#61;&#61; std::signer::address_of(framework);<br /><b>include</b> <a href="config_buffer.md#0x1_config_buffer_OnNewEpochRequirement">config_buffer::OnNewEpochRequirement</a>&lt;<a href="gas_schedule.md#0x1_gas_schedule_GasScheduleV2">GasScheduleV2</a>&gt;;<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>



<a id="@Specification_1_set_storage_gas_config"></a>

### Function `set_storage_gas_config`


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config">set_storage_gas_config</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)<br /></code></pre>




<pre><code><b>pragma</b> verify_duration_estimate &#61; 600;<br /><b>requires</b> <b>exists</b>&lt;<a href="stake.md#0x1_stake_ValidatorFees">stake::ValidatorFees</a>&gt;(@aptos_framework);<br /><b>requires</b> <b>exists</b>&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br /><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br /><b>include</b> <a href="transaction_fee.md#0x1_transaction_fee_RequiresCollectedFeesPerValueLeqBlockAptosSupply">transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply</a>;<br /><b>include</b> <a href="staking_config.md#0x1_staking_config_StakingRewardsConfigRequirement">staking_config::StakingRewardsConfigRequirement</a>;<br /><b>aborts_if</b> !<b>exists</b>&lt;StorageGasConfig&gt;(@aptos_framework);<br /><b>ensures</b> <b>global</b>&lt;StorageGasConfig&gt;(@aptos_framework) &#61;&#61; config;<br /></code></pre>




<pre><code><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_set_storage_gas_config_for_next_epoch"></a>

### Function `set_storage_gas_config_for_next_epoch`


<pre><code><b>public</b> <b>fun</b> <a href="gas_schedule.md#0x1_gas_schedule_set_storage_gas_config_for_next_epoch">set_storage_gas_config_for_next_epoch</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, config: <a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>)<br /></code></pre>




<pre><code><b>include</b> <a href="system_addresses.md#0x1_system_addresses_AbortsIfNotAptosFramework">system_addresses::AbortsIfNotAptosFramework</a>&#123; <a href="account.md#0x1_account">account</a>: aptos_framework &#125;;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="storage_gas.md#0x1_storage_gas_StorageGasConfig">storage_gas::StorageGasConfig</a>&gt;(@aptos_framework);<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
