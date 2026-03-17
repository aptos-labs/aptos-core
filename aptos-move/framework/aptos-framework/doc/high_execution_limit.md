
<a id="0x1_high_execution_limit"></a>

# Module `0x1::high_execution_limit`

Manages the per-epoch slot counter for high-execution-limit transactions.

A fixed number of transactions per epoch may opt into a high-limit gas tier
by paying a flat premium. Higher limits are allocated on the first-come-first-served
basis. Whether transaction succeeds or not, the extended compute limit is considered
to be used.


-  [Resource `HighExecutionLimitConfig`](#0x1_high_execution_limit_HighExecutionLimitConfig)
-  [Function `initialize`](#0x1_high_execution_limit_initialize)
-  [Function `update_max_per_epoch`](#0x1_high_execution_limit_update_max_per_epoch)
-  [Function `on_new_epoch`](#0x1_high_execution_limit_on_new_epoch)
-  [Function `is_high_execution_limit_available`](#0x1_high_execution_limit_is_high_execution_limit_available)
-  [Function `record_used_high_execution_limit`](#0x1_high_execution_limit_record_used_high_execution_limit)
    -  [Precondition](#@Precondition_0)


<pre><code><b>use</b> <a href="aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_high_execution_limit_HighExecutionLimitConfig"></a>

## Resource `HighExecutionLimitConfig`



<pre><code><b>struct</b> <a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>available: <a href="aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u64&gt;</code>
</dt>
<dd>
 Counter to track how many transactions can still use higher execution
 limits in this epoch.
</dd>
<dt>
<code>max_per_epoch: u64</code>
</dt>
<dd>
 Maximum number of allowed high-limit transactions (per epoch).
</dd>
</dl>


</details>

<a id="0x1_high_execution_limit_initialize"></a>

## Function `initialize`

Called once during genesis or governance to install the resource.


<pre><code><b>public</b> <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_per_epoch: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_per_epoch: u64) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(aptos_framework, <a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a> {
            available: <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator_with_value">aggregator_v2::create_aggregator_with_value</a>(max_per_epoch, max_per_epoch),
            max_per_epoch,
        });
    }
}
</code></pre>



</details>

<a id="0x1_high_execution_limit_update_max_per_epoch"></a>

## Function `update_max_per_epoch`

For governance to update maximum number of allowed high-limit transactions per epoch.


<pre><code><b>public</b> <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_update_max_per_epoch">update_max_per_epoch</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_per_epoch: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_update_max_per_epoch">update_max_per_epoch</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, max_per_epoch: u64) <b>acquires</b> <a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_is_high_execution_limit_transactions_enabled">features::is_high_execution_limit_transactions_enabled</a>()) {
        <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a>&gt;(@aptos_framework);
        config.available = <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator_with_value">aggregator_v2::create_aggregator_with_value</a>(max_per_epoch, max_per_epoch);
        config.max_per_epoch = max_per_epoch;
    }
}
</code></pre>



</details>

<a id="0x1_high_execution_limit_on_new_epoch"></a>

## Function `on_new_epoch`

Called at each epoch boundary to reset the counter.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_on_new_epoch">on_new_epoch</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_on_new_epoch">on_new_epoch</a>() <b>acquires</b> <a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a> {
    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a>&gt;(@aptos_framework);
    <b>let</b> max_per_epoch = config.max_per_epoch;
    config.available = <a href="aggregator_v2.md#0x1_aggregator_v2_create_aggregator_with_value">aggregator_v2::create_aggregator_with_value</a>(max_per_epoch, max_per_epoch);
}
</code></pre>



</details>

<a id="0x1_high_execution_limit_is_high_execution_limit_available"></a>

## Function `is_high_execution_limit_available`

Returns true if the high execution limit is available. Only called in prologue.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_is_high_execution_limit_available">is_high_execution_limit_available</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_is_high_execution_limit_available">is_high_execution_limit_available</a>(): bool <b>acquires</b> <a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a> {
    <b>let</b> config = <b>borrow_global</b>&lt;<a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a>&gt;(@aptos_framework);
    config.available.is_at_least(1)
}
</code></pre>



</details>

<a id="0x1_high_execution_limit_record_used_high_execution_limit"></a>

## Function `record_used_high_execution_limit`

Decrements the counter marking high-execution limit as used. Only called in epilogue.


<a id="@Precondition_0"></a>

### Precondition


Prologue must check that the high execution limit is available.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_record_used_high_execution_limit">record_used_high_execution_limit</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="high_execution_limit.md#0x1_high_execution_limit_record_used_high_execution_limit">record_used_high_execution_limit</a>() <b>acquires</b> <a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a> {
    <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="high_execution_limit.md#0x1_high_execution_limit_HighExecutionLimitConfig">HighExecutionLimitConfig</a>&gt;(@aptos_framework);
    config.available.sub(1);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
