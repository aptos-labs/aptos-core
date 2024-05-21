
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


<pre><code>use 0x1::chain_status;<br/>use 0x1::config_buffer;<br/>use 0x1::error;<br/>use 0x1::reconfiguration;<br/>use 0x1::storage_gas;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/>use 0x1::util;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_gas_schedule_GasEntry"></a>

## Struct `GasEntry`



<pre><code>struct GasEntry has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>key: string::String</code>
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



<pre><code>struct GasSchedule has copy, drop, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: vector&lt;gas_schedule::GasEntry&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_gas_schedule_GasScheduleV2"></a>

## Resource `GasScheduleV2`



<pre><code>struct GasScheduleV2 has copy, drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>feature_version: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>entries: vector&lt;gas_schedule::GasEntry&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_gas_schedule_EINVALID_GAS_FEATURE_VERSION"></a>



<pre><code>const EINVALID_GAS_FEATURE_VERSION: u64 &#61; 2;<br/></code></pre>



<a id="0x1_gas_schedule_EINVALID_GAS_SCHEDULE"></a>

The provided gas schedule bytes are empty or invalid


<pre><code>const EINVALID_GAS_SCHEDULE: u64 &#61; 1;<br/></code></pre>



<a id="0x1_gas_schedule_initialize"></a>

## Function `initialize`

Only called during genesis.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(!vector::is_empty(&amp;gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));<br/><br/>    // TODO(Gas): check if gas schedule is consistent<br/>    let gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);<br/>    move_to&lt;GasScheduleV2&gt;(aptos_framework, gas_schedule);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_gas_schedule_set_gas_schedule"></a>

## Function `set_gas_schedule`

Deprecated by <code>set_for_next_epoch()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code>public fun set_gas_schedule(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_gas_schedule(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;) acquires GasSchedule, GasScheduleV2 &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(!vector::is_empty(&amp;gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));<br/>    chain_status::assert_genesis();<br/><br/>    if (exists&lt;GasScheduleV2&gt;(@aptos_framework)) &#123;<br/>        let gas_schedule &#61; borrow_global_mut&lt;GasScheduleV2&gt;(@aptos_framework);<br/>        let new_gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);<br/>        assert!(new_gas_schedule.feature_version &gt;&#61; gas_schedule.feature_version,<br/>            error::invalid_argument(EINVALID_GAS_FEATURE_VERSION));<br/>        // TODO(Gas): check if gas schedule is consistent<br/>        &#42;gas_schedule &#61; new_gas_schedule;<br/>    &#125;<br/>    else &#123;<br/>        if (exists&lt;GasSchedule&gt;(@aptos_framework)) &#123;<br/>            _ &#61; move_from&lt;GasSchedule&gt;(@aptos_framework);<br/>        &#125;;<br/>        let new_gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);<br/>        // TODO(Gas): check if gas schedule is consistent<br/>        move_to&lt;GasScheduleV2&gt;(aptos_framework, new_gas_schedule);<br/>    &#125;;<br/><br/>    // Need to trigger reconfiguration so validator nodes can sync on the updated gas schedule.<br/>    reconfiguration::reconfigure();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_gas_schedule_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

Set the gas schedule for the next epoch, typically called by on-chain governance.
Abort if the version of the given schedule is lower than the current version.

Example usage:
```
aptos_framework::gas_schedule::set_for_next_epoch(&framework_signer, some_gas_schedule_blob);
aptos_framework::aptos_governance::reconfigure(&framework_signer);
```


<pre><code>public fun set_for_next_epoch(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;) acquires GasScheduleV2 &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    assert!(!vector::is_empty(&amp;gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));<br/>    let new_gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);<br/>    if (exists&lt;GasScheduleV2&gt;(@aptos_framework)) &#123;<br/>        let cur_gas_schedule &#61; borrow_global&lt;GasScheduleV2&gt;(@aptos_framework);<br/>        assert!(<br/>            new_gas_schedule.feature_version &gt;&#61; cur_gas_schedule.feature_version,<br/>            error::invalid_argument(EINVALID_GAS_FEATURE_VERSION)<br/>        );<br/>    &#125;;<br/>    config_buffer::upsert(new_gas_schedule);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_gas_schedule_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>GasScheduleV2</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires GasScheduleV2 &#123;<br/>    system_addresses::assert_aptos_framework(framework);<br/>    if (config_buffer::does_exist&lt;GasScheduleV2&gt;()) &#123;<br/>        let new_gas_schedule &#61; config_buffer::extract&lt;GasScheduleV2&gt;();<br/>        if (exists&lt;GasScheduleV2&gt;(@aptos_framework)) &#123;<br/>            &#42;borrow_global_mut&lt;GasScheduleV2&gt;(@aptos_framework) &#61; new_gas_schedule;<br/>        &#125; else &#123;<br/>            move_to(framework, new_gas_schedule);<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_gas_schedule_set_storage_gas_config"></a>

## Function `set_storage_gas_config`



<pre><code>public fun set_storage_gas_config(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_storage_gas_config(aptos_framework: &amp;signer, config: StorageGasConfig) &#123;<br/>    storage_gas::set_config(aptos_framework, config);<br/>    // Need to trigger reconfiguration so the VM is guaranteed to load the new gas fee starting from the next<br/>    // transaction.<br/>    reconfiguration::reconfigure();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_gas_schedule_set_storage_gas_config_for_next_epoch"></a>

## Function `set_storage_gas_config_for_next_epoch`



<pre><code>public fun set_storage_gas_config_for_next_epoch(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_storage_gas_config_for_next_epoch(aptos_framework: &amp;signer, config: StorageGasConfig) &#123;<br/>    storage_gas::set_config(aptos_framework, config);<br/>&#125;<br/></code></pre>



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


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(aptos_framework);<br/>// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>// This enforces <a id="high-level-req-3.3" href="#high-level-req">high-level requirement 3</a>:
aborts_if len(gas_schedule_blob) &#61;&#61; 0;<br/>aborts_if exists&lt;GasScheduleV2&gt;(addr);<br/>ensures exists&lt;GasScheduleV2&gt;(addr);<br/></code></pre>



<a id="@Specification_1_set_gas_schedule"></a>

### Function `set_gas_schedule`


<pre><code>public fun set_gas_schedule(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 600;<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>requires chain_status::is_genesis();<br/>include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>include staking_config::StakingRewardsConfigRequirement;<br/>// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a>:
aborts_if len(gas_schedule_blob) &#61;&#61; 0;<br/>let new_gas_schedule &#61; util::spec_from_bytes&lt;GasScheduleV2&gt;(gas_schedule_blob);<br/>let gas_schedule &#61; global&lt;GasScheduleV2&gt;(@aptos_framework);<br/>// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
aborts_if exists&lt;GasScheduleV2&gt;(@aptos_framework) &amp;&amp; new_gas_schedule.feature_version &lt; gas_schedule.feature_version;<br/>ensures exists&lt;GasScheduleV2&gt;(signer::address_of(aptos_framework));<br/>ensures global&lt;GasScheduleV2&gt;(@aptos_framework) &#61;&#61; new_gas_schedule;<br/></code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code>public fun set_for_next_epoch(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>include config_buffer::SetForNextEpochAbortsIf &#123;<br/>    account: aptos_framework,<br/>    config: gas_schedule_blob<br/>&#125;;<br/>let new_gas_schedule &#61; util::spec_from_bytes&lt;GasScheduleV2&gt;(gas_schedule_blob);<br/>let cur_gas_schedule &#61; global&lt;GasScheduleV2&gt;(@aptos_framework);<br/>aborts_if exists&lt;GasScheduleV2&gt;(@aptos_framework) &amp;&amp; new_gas_schedule.feature_version &lt; cur_gas_schedule.feature_version;<br/></code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)<br/></code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);<br/>include config_buffer::OnNewEpochRequirement&lt;GasScheduleV2&gt;;<br/>aborts_if false;<br/></code></pre>



<a id="@Specification_1_set_storage_gas_config"></a>

### Function `set_storage_gas_config`


<pre><code>public fun set_storage_gas_config(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)<br/></code></pre>




<pre><code>pragma verify_duration_estimate &#61; 600;<br/>requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);<br/>requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);<br/>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;<br/>include staking_config::StakingRewardsConfigRequirement;<br/>aborts_if !exists&lt;StorageGasConfig&gt;(@aptos_framework);<br/>ensures global&lt;StorageGasConfig&gt;(@aptos_framework) &#61;&#61; config;<br/></code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>aborts_if !exists&lt;storage_gas::StorageGasConfig&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_set_storage_gas_config_for_next_epoch"></a>

### Function `set_storage_gas_config_for_next_epoch`


<pre><code>public fun set_storage_gas_config_for_next_epoch(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)<br/></code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;<br/>aborts_if !exists&lt;storage_gas::StorageGasConfig&gt;(@aptos_framework);<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
