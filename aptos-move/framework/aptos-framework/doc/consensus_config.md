
<a id="0x1_consensus_config"></a>

# Module `0x1::consensus_config`

Maintains the consensus config for the blockchain. The config is stored in a
Reconfiguration, and may be updated by root.


-  [Resource `ConsensusConfig`](#0x1_consensus_config_ConsensusConfig)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_consensus_config_initialize)
-  [Function `set`](#0x1_consensus_config_set)
-  [Function `set_for_next_epoch`](#0x1_consensus_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_consensus_config_on_new_epoch)
-  [Function `validator_txn_enabled`](#0x1_consensus_config_validator_txn_enabled)
-  [Function `validator_txn_enabled_internal`](#0x1_consensus_config_validator_txn_enabled_internal)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `set`](#@Specification_1_set)
    -  [Function `set_for_next_epoch`](#@Specification_1_set_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)
    -  [Function `validator_txn_enabled`](#@Specification_1_validator_txn_enabled)
    -  [Function `validator_txn_enabled_internal`](#@Specification_1_validator_txn_enabled_internal)


<pre><code>use 0x1::chain_status;
use 0x1::config_buffer;
use 0x1::error;
use 0x1::reconfiguration;
use 0x1::system_addresses;
</code></pre>



<a id="0x1_consensus_config_ConsensusConfig"></a>

## Resource `ConsensusConfig`



<pre><code>struct ConsensusConfig has drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>config: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_consensus_config_EINVALID_CONFIG"></a>

The provided on chain config bytes are empty or invalid


<pre><code>const EINVALID_CONFIG: u64 &#61; 1;
</code></pre>



<a id="0x1_consensus_config_initialize"></a>

## Function `initialize`

Publishes the ConsensusConfig config.


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, config: vector&lt;u8&gt;) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    assert!(vector::length(&amp;config) &gt; 0, error::invalid_argument(EINVALID_CONFIG));
    move_to(aptos_framework, ConsensusConfig &#123; config &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_consensus_config_set"></a>

## Function `set`

Deprecated by <code>set_for_next_epoch()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;) acquires ConsensusConfig &#123;
    system_addresses::assert_aptos_framework(account);
    chain_status::assert_genesis();
    assert!(vector::length(&amp;config) &gt; 0, error::invalid_argument(EINVALID_CONFIG));

    let config_ref &#61; &amp;mut borrow_global_mut&lt;ConsensusConfig&gt;(@aptos_framework).config;
    &#42;config_ref &#61; config;

    // Need to trigger reconfiguration so validator nodes can sync on the updated configs.
    reconfiguration::reconfigure();
&#125;
</code></pre>



</details>

<a id="0x1_consensus_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on-chain governance to update on-chain consensus configs for the next epoch.
Example usage:
```
aptos_framework::consensus_config::set_for_next_epoch(&framework_signer, some_config_bytes);
aptos_framework::aptos_governance::reconfigure(&framework_signer);
```


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;) &#123;
    system_addresses::assert_aptos_framework(account);
    assert!(vector::length(&amp;config) &gt; 0, error::invalid_argument(EINVALID_CONFIG));
    std::config_buffer::upsert&lt;ConsensusConfig&gt;(ConsensusConfig &#123;config&#125;);
&#125;
</code></pre>



</details>

<a id="0x1_consensus_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>ConsensusConfig</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires ConsensusConfig &#123;
    system_addresses::assert_aptos_framework(framework);
    if (config_buffer::does_exist&lt;ConsensusConfig&gt;()) &#123;
        let new_config &#61; config_buffer::extract&lt;ConsensusConfig&gt;();
        if (exists&lt;ConsensusConfig&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;ConsensusConfig&gt;(@aptos_framework) &#61; new_config;
        &#125; else &#123;
            move_to(framework, new_config);
        &#125;;
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_consensus_config_validator_txn_enabled"></a>

## Function `validator_txn_enabled`



<pre><code>public fun validator_txn_enabled(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun validator_txn_enabled(): bool acquires ConsensusConfig &#123;
    let config_bytes &#61; borrow_global&lt;ConsensusConfig&gt;(@aptos_framework).config;
    validator_txn_enabled_internal(config_bytes)
&#125;
</code></pre>



</details>

<a id="0x1_consensus_config_validator_txn_enabled_internal"></a>

## Function `validator_txn_enabled_internal`



<pre><code>fun validator_txn_enabled_internal(config_bytes: vector&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun validator_txn_enabled_internal(config_bytes: vector&lt;u8&gt;): bool;
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
<td>During genesis, the Aptos framework account should be assigned the consensus config resource.</td>
<td>Medium</td>
<td>The consensus_config::initialize function calls the assert_aptos_framework function to ensure that the signer is the aptos_framework and then assigns the ConsensusConfig resource to it.</td>
<td>Formally verified via <a href="#high-level-req-1">initialize</a>.</td>
</tr>

<tr>
<td>2</td>
<td>Only aptos framework account is allowed to update the consensus configuration.</td>
<td>Medium</td>
<td>The consensus_config::set function ensures that the signer is aptos_framework.</td>
<td>Formally verified via <a href="#high-level-req-2">set</a>.</td>
</tr>

<tr>
<td>3</td>
<td>Only a valid configuration can be used during initialization and update.</td>
<td>Medium</td>
<td>Both the initialize and set functions validate the config by ensuring its length to be greater than 0.</td>
<td>Formally verified via <a href="#high-level-req-3.1">initialize</a> and <a href="#high-level-req-3.2">set</a>.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;ConsensusConfig&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public(friend) fun initialize(aptos_framework: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>


Ensure caller is admin.
Aborts if StateStorageUsage already exists.


<pre><code>let addr &#61; signer::address_of(aptos_framework);
// This enforces <a id="high-level-req-1" href="#high-level-req">high-level requirement 1</a>:
aborts_if !system_addresses::is_aptos_framework_address(addr);
aborts_if exists&lt;ConsensusConfig&gt;(@aptos_framework);
// This enforces <a id="high-level-req-3.1" href="#high-level-req">high-level requirement 3</a>:
aborts_if !(len(config) &gt; 0);
ensures global&lt;ConsensusConfig&gt;(addr) &#61;&#61; ConsensusConfig &#123; config &#125;;
</code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>


Ensure the caller is admin and <code>ConsensusConfig</code> should be existed.
When setting now time must be later than last_reconfiguration_time.


<pre><code>pragma verify_duration_estimate &#61; 600;
include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
include staking_config::StakingRewardsConfigRequirement;
let addr &#61; signer::address_of(account);
// This enforces <a id="high-level-req-2" href="#high-level-req">high-level requirement 2</a>:
aborts_if !system_addresses::is_aptos_framework_address(addr);
aborts_if !exists&lt;ConsensusConfig&gt;(@aptos_framework);
// This enforces <a id="high-level-req-3.2" href="#high-level-req">high-level requirement 3</a>:
aborts_if !(len(config) &gt; 0);
requires chain_status::is_genesis();
requires timestamp::spec_now_microseconds() &gt;&#61; reconfiguration::last_reconfiguration_time();
requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
requires exists&lt;CoinInfo&lt;AptosCoin&gt;&gt;(@aptos_framework);
ensures global&lt;ConsensusConfig&gt;(@aptos_framework).config &#61;&#61; config;
</code></pre>



<a id="@Specification_1_set_for_next_epoch"></a>

### Function `set_for_next_epoch`


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>




<pre><code>include config_buffer::SetForNextEpochAbortsIf;
</code></pre>



<a id="@Specification_1_on_new_epoch"></a>

### Function `on_new_epoch`


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>




<pre><code>requires @aptos_framework &#61;&#61; std::signer::address_of(framework);
include config_buffer::OnNewEpochRequirement&lt;ConsensusConfig&gt;;
aborts_if false;
</code></pre>



<a id="@Specification_1_validator_txn_enabled"></a>

### Function `validator_txn_enabled`


<pre><code>public fun validator_txn_enabled(): bool
</code></pre>




<pre><code>pragma opaque;
aborts_if !exists&lt;ConsensusConfig&gt;(@aptos_framework);
ensures [abstract] result &#61;&#61; spec_validator_txn_enabled_internal(global&lt;ConsensusConfig&gt;(@aptos_framework).config);
</code></pre>



<a id="@Specification_1_validator_txn_enabled_internal"></a>

### Function `validator_txn_enabled_internal`


<pre><code>fun validator_txn_enabled_internal(config_bytes: vector&lt;u8&gt;): bool
</code></pre>




<pre><code>pragma opaque;
ensures [abstract] result &#61;&#61; spec_validator_txn_enabled_internal(config_bytes);
</code></pre>




<a id="0x1_consensus_config_spec_validator_txn_enabled_internal"></a>


<pre><code>fun spec_validator_txn_enabled_internal(config_bytes: vector&lt;u8&gt;): bool;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
