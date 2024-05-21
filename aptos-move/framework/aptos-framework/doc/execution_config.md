
<a id="0x1_execution_config"></a>

# Module `0x1::execution_config`

Maintains the execution config for the blockchain. The config is stored in a
Reconfiguration, and may be updated by root.


-  [Resource `ExecutionConfig`](#0x1_execution_config_ExecutionConfig)
-  [Constants](#@Constants_0)
-  [Function `set`](#0x1_execution_config_set)
-  [Function `set_for_next_epoch`](#0x1_execution_config_set_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_execution_config_on_new_epoch)
-  [Specification](#@Specification_1)
    -  [Function `set`](#@Specification_1_set)
    -  [Function `set_for_next_epoch`](#@Specification_1_set_for_next_epoch)
    -  [Function `on_new_epoch`](#@Specification_1_on_new_epoch)


<pre><code>use 0x1::chain_status;
use 0x1::config_buffer;
use 0x1::error;
use 0x1::reconfiguration;
use 0x1::system_addresses;
</code></pre>



<a id="0x1_execution_config_ExecutionConfig"></a>

## Resource `ExecutionConfig`



<pre><code>struct ExecutionConfig has drop, store, key
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


<a id="0x1_execution_config_EINVALID_CONFIG"></a>

The provided on chain config bytes are empty or invalid


<pre><code>const EINVALID_CONFIG: u64 &#61; 1;
</code></pre>



<a id="0x1_execution_config_set"></a>

## Function `set`

Deprecated by <code>set_for_next_epoch()</code>.

WARNING: calling this while randomness is enabled will trigger a new epoch without randomness!

TODO: update all the tests that reference this function, then disable this function.


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;) acquires ExecutionConfig &#123;
    system_addresses::assert_aptos_framework(account);
    chain_status::assert_genesis();

    assert!(vector::length(&amp;config) &gt; 0, error::invalid_argument(EINVALID_CONFIG));

    if (exists&lt;ExecutionConfig&gt;(@aptos_framework)) &#123;
        let config_ref &#61; &amp;mut borrow_global_mut&lt;ExecutionConfig&gt;(@aptos_framework).config;
        &#42;config_ref &#61; config;
    &#125; else &#123;
        move_to(account, ExecutionConfig &#123; config &#125;);
    &#125;;
    // Need to trigger reconfiguration so validator nodes can sync on the updated configs.
    reconfiguration::reconfigure();
&#125;
</code></pre>



</details>

<a id="0x1_execution_config_set_for_next_epoch"></a>

## Function `set_for_next_epoch`

This can be called by on-chain governance to update on-chain execution configs for the next epoch.
Example usage:
```
aptos_framework::execution_config::set_for_next_epoch(&framework_signer, some_config_bytes);
aptos_framework::aptos_governance::reconfigure(&framework_signer);
```


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun set_for_next_epoch(account: &amp;signer, config: vector&lt;u8&gt;) &#123;
    system_addresses::assert_aptos_framework(account);
    assert!(vector::length(&amp;config) &gt; 0, error::invalid_argument(EINVALID_CONFIG));
    config_buffer::upsert(ExecutionConfig &#123; config &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_execution_config_on_new_epoch"></a>

## Function `on_new_epoch`

Only used in reconfigurations to apply the pending <code>ExecutionConfig</code>, if there is any.


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_new_epoch(framework: &amp;signer) acquires ExecutionConfig &#123;
    system_addresses::assert_aptos_framework(framework);
    if (config_buffer::does_exist&lt;ExecutionConfig&gt;()) &#123;
        let config &#61; config_buffer::extract&lt;ExecutionConfig&gt;();
        if (exists&lt;ExecutionConfig&gt;(@aptos_framework)) &#123;
            &#42;borrow_global_mut&lt;ExecutionConfig&gt;(@aptos_framework) &#61; config;
        &#125; else &#123;
            move_to(framework, config);
        &#125;;
    &#125;
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>pragma verify &#61; true;
pragma aborts_if_is_strict;
</code></pre>



<a id="@Specification_1_set"></a>

### Function `set`


<pre><code>public fun set(account: &amp;signer, config: vector&lt;u8&gt;)
</code></pre>


Ensure the caller is admin
When setting now time must be later than last_reconfiguration_time.


<pre><code>pragma verify_duration_estimate &#61; 600;
let addr &#61; signer::address_of(account);
include transaction_fee::RequiresCollectedFeesPerValueLeqBlockAptosSupply;
requires chain_status::is_genesis();
requires exists&lt;stake::ValidatorFees&gt;(@aptos_framework);
requires exists&lt;staking_config::StakingRewardsConfig&gt;(@aptos_framework);
requires len(config) &gt; 0;
include features::spec_periodical_reward_rate_decrease_enabled() &#61;&#61;&gt; staking_config::StakingRewardsConfigEnabledRequirement;
include aptos_coin::ExistsAptosCoin;
requires system_addresses::is_aptos_framework_address(addr);
requires timestamp::spec_now_microseconds() &gt;&#61; reconfiguration::last_reconfiguration_time();
ensures exists&lt;ExecutionConfig&gt;(@aptos_framework);
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
include config_buffer::OnNewEpochRequirement&lt;ExecutionConfig&gt;;
aborts_if false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
