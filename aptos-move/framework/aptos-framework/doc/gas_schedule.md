
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


<pre><code>use 0x1::chain_status;
use 0x1::config_buffer;
use 0x1::error;
use 0x1::reconfiguration;
use 0x1::storage_gas;
use 0x1::string;
use 0x1::system_addresses;
use 0x1::util;
use 0x1::vector;
</code></pre>



<a id="0x1_gas_schedule_GasEntry"></a>

## Struct `GasEntry`



<pre><code>struct GasEntry has copy, drop, store
</code></pre>



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



<pre><code>struct GasSchedule has copy, drop, key
</code></pre>



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



<pre><code>struct GasScheduleV2 has copy, drop, store, key
</code></pre>



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



<pre><code>const EINVALID_GAS_FEATURE_VERSION: u64 &#61; 2;
</code></pre>



<a id="0x1_gas_schedule_EINVALID_GAS_SCHEDULE"></a>

The provided gas schedule bytes are empty or invalid


<pre><code>const EINVALID_GAS_SCHEDULE: u64 &#61; 1;
</code></pre>



<a id="0x1_gas_schedule_initialize"></a>

## Function `initialize`

Only called during genesis.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(!vector::is_empty(&amp;gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));

    // TODO(Gas): check if gas schedule is consistent
    let gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);
    move_to&lt;GasScheduleV2&gt;(aptos_framework, gas_schedule);
&#125;
</code></pre>



</details>

<a id="0x1_gas_schedule_set_gas_schedule"></a>

## Function `set_gas_schedule`

Deprecated by <code>set_for_next_epoch()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code>public fun set_gas_schedule(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_gas_schedule(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;) acquires GasSchedule, GasScheduleV2 &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(!vector::is_empty(&amp;gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));
    chain_status::assert_genesis();

    if (exists&lt;GasScheduleV2&gt;(@aptos_framework)) &#123;
        let gas_schedule &#61; borrow_global_mut&lt;GasScheduleV2&gt;(@aptos_framework);
        let new_gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);
        assert!(new_gas_schedule.feature_version &gt;&#61; gas_schedule.feature_version,
            error::invalid_argument(EINVALID_GAS_FEATURE_VERSION));
        // TODO(Gas): check if gas schedule is consistent
        &#42;gas_schedule &#61; new_gas_schedule;
    &#125;
    else &#123;
        if (exists&lt;GasSchedule&gt;(@aptos_framework)) &#123;
            _ &#61; move_from&lt;GasSchedule&gt;(@aptos_framework);
        &#125;;
        let new_gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);
        // TODO(Gas): check if gas schedule is consistent
        move_to&lt;GasScheduleV2&gt;(aptos_framework, new_gas_schedule);
    &#125;;

    // Need to trigger reconfiguration so validator nodes can sync on the updated gas schedule.
    reconfiguration::reconfigure();
&#125;
</code></pre>



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


<pre><code>public fun set_for_next_epoch(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;) acquires GasScheduleV2 &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(!vector::is_empty(&amp;gas_schedule_blob), error::invalid_argument(EINVALID_GAS_SCHEDULE));
    let new_gas_schedule: GasScheduleV2 &#61; from_bytes(gas_schedule_blob);
    if (exists&lt;GasScheduleV2&gt;(@aptos_framework)) &#123;
        let cur_gas_schedule &#61; borrow_global&lt;GasScheduleV2&gt;(@aptos_framework);
        assert!(
            new_gas_schedule.feature_version &gt;&#61; cur_gas_schedule.feature_version,
            error::invalid_argument(EINVALID_GAS_FEATURE_VERSION)
        );
    &#125;;
    config_buffer::upsert(new_gas_schedule);
&#125;
</code></pre>



</details>

<a id="0x1_gas_schedule_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>GasScheduleV2</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires GasScheduleV2 &#123;
    system_addresses::assert_aptos_framework(framework);
    if (config_buffer::does_exist&lt;GasScheduleV2&gt;()) &#123;
        let new_gas_schedule &#61; config_buffer::extract&lt;GasScheduleV2&gt;();
        if (exists&lt;GasScheduleV2&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;GasScheduleV2&gt;(@aptos_framework) &#61; new_gas_schedule;
        &#125; else &#123;
            move_to(framework, new_gas_schedule);
        &#125;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_gas_schedule_set_storage_gas_config"></a>

## Function `set_storage_gas_config`



<pre><code>public fun set_storage_gas_config(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_storage_gas_config(aptos_framework: &amp;signer, config: StorageGasConfig) &#123;
    storage_gas::set_config(aptos_framework, config);
    // Need to trigger reconfiguration so the VM is guaranteed to load the new gas fee starting from the next
    // transaction.
    reconfiguration::reconfigure();
&#125;
</code></pre>



</details>

<a id="0x1_gas_schedule_set_storage_gas_config_for_next_epoch"></a>

## Function `set_storage_gas_config_for_next_epoch`



<pre><code>public fun set_storage_gas_config_for_next_epoch(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_storage_gas_config_for_next_epoch(aptos_framework: &amp;signer, config: StorageGasConfig) &#123;
    storage_gas::set_config(aptos_framework, config);
&#125;
</code></pre>



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


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)
</code></pre>




<pre><code>let addr &#61; signer::address_of(aptos_framework);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;
// This enforces <a id="high-level-req-3.3" href="#high-level-req">high-level requirement 3</a>:
aborts_if len(gas_schedule_blob) &#61;&#61; 0;
aborts_if exists&lt;GasScheduleV2&gt;(addr);
ensures exists&lt;GasScheduleV2&gt;(addr);
</code></pre>



<a id="@Specification_1_set_gas_schedule"></a>

### Function `set_gas_schedule`


<pre><code>public fun set_gas_schedule(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 600;
requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
requires chain_status::is_genesis();
include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
include staking_config::StakingRewardsConfigRequirement;
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;
// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a>:
aborts_if len(gas_schedule_blob) &#61;&#61; 0;
let new_gas_schedule &#61; util::spec_from_bytes&lt;GasScheduleV2&gt;(gas_schedule_blob);
let gas_schedule &#61; global&lt;GasScheduleV2&gt;(@aptos_framework);
// This enforces <a id="high-level-req-4" href="#high-level-req">high-level requirement 4</a>:
aborts_if exists&lt;GasScheduleV2&gt;(@aptos_framework) &amp;&amp; new_gas_schedule.feature_version &lt; gas_schedule.feature_version;
ensures exists&lt;GasScheduleV2&gt;(signer::address_of(aptos_framework));
ensures global&lt;GasScheduleV2&gt;(@aptos_framework) &#61;&#61; new_gas_schedule;
</code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code>public fun set_for_next_epoch(aptos_framework: &amp;signer, gas_schedule_blob: vector&lt;u8&gt;)
</code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;
include config_buffer::SetForNextEpochAbortsIf &#123;
    account: aptos_framework,
    config: gas_schedule_blob
&#125;;
let new_gas_schedule &#61; util::spec_from_bytes&lt;GasScheduleV2&gt;(gas_schedule_blob);
let cur_gas_schedule &#61; global&lt;GasScheduleV2&gt;(@aptos_framework);
aborts_if exists&lt;GasScheduleV2&gt;(@aptos_framework) &amp;&amp; new_gas_schedule.feature_version &lt; cur_gas_schedule.feature_version;
</code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);
include config_buffer::OnNewEpochRequirement&lt;GasScheduleV2&gt;;
aborts_if false;
</code></pre>



<a id="@Specification_1_set_storage_gas_config"></a>

### Function `set_storage_gas_config`


<pre><code>public fun set_storage_gas_config(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)
</code></pre>




<pre><code>pragma verify_duration_estimate &#61; 600;
requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;
include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
include staking_config::StakingRewardsConfigRequirement;
aborts_if !exists&lt;StorageGasConfig&gt;(@aptos_framework);
ensures global&lt;StorageGasConfig&gt;(@aptos_framework) &#61;&#61; config;
</code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;
aborts_if !exists&lt;storage_gas::StorageGasConfig&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_set_storage_gas_config_for_next_epoch"></a>

### Function `set_storage_gas_config_for_next_epoch`


<pre><code>public fun set_storage_gas_config_for_next_epoch(aptos_framework: &amp;signer, config: storage_gas::StorageGasConfig)
</code></pre>




<pre><code>include system_addresses::AbortsIfNotAptosFramework&#123; account: aptos_framework &#125;;
aborts_if !exists&lt;storage_gas::StorageGasConfig&gt;(@aptos_framework);
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
