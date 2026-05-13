
<a id="0x7_position_counts"></a>

# Module `0x7::position_counts`

Per-exchange position-count ceiling, enforced atomically via AggregatorV2.

One <code><a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a></code> resource lives at <code>@aptos_experimental</code>, holding a
table keyed by <code>exchange_id</code>. <code><a href="native_position.md#0x7_native_position_register">native_position::register</a>()</code> allocates a
counter with <code>max = initial_max</code> when an exchange first registers;
<code>create_position</code> / <code>remove_position</code> bump / decrement it; governance can
tune <code>max</code> via <code>update_ceiling</code>.

Delayed-field semantics on <code>AggregatorV2&lt;u64&gt;</code> mean concurrent
<code>try_add</code> / <code>sub</code> calls on the same counter don't conflict in Block-STM,
as long as the bound isn't hit.


-  [Resource `PositionCounters`](#0x7_position_counts_PositionCounters)
-  [Constants](#@Constants_0)
-  [Function `init_module`](#0x7_position_counts_init_module)
-  [Function `initialize_for_genesis`](#0x7_position_counts_initialize_for_genesis)
-  [Function `allocate_counter`](#0x7_position_counts_allocate_counter)
-  [Function `counter_exists`](#0x7_position_counts_counter_exists)
-  [Function `try_add`](#0x7_position_counts_try_add)
-  [Function `sub`](#0x7_position_counts_sub)
-  [Function `update_ceiling`](#0x7_position_counts_update_ceiling)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2">0x1::aggregator_v2</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x7_position_counts_PositionCounters"></a>

## Resource `PositionCounters`



<pre><code><b>struct</b> <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>counts: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u32, <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_Aggregator">aggregator_v2::Aggregator</a>&lt;u64&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_position_counts_ENOT_INITIALIZED"></a>

PositionCounters resource has not been initialized yet.


<pre><code><b>const</b> <a href="position_counts.md#0x7_position_counts_ENOT_INITIALIZED">ENOT_INITIALIZED</a>: u64 = 2;
</code></pre>



<a id="0x7_position_counts_ECOUNTER_ALREADY_ALLOCATED"></a>

Counter already allocated for this exchange_id.


<pre><code><b>const</b> <a href="position_counts.md#0x7_position_counts_ECOUNTER_ALREADY_ALLOCATED">ECOUNTER_ALREADY_ALLOCATED</a>: u64 = 4;
</code></pre>



<a id="0x7_position_counts_ECOUNTER_NOT_FOUND"></a>

No counter has been allocated for this exchange_id.


<pre><code><b>const</b> <a href="position_counts.md#0x7_position_counts_ECOUNTER_NOT_FOUND">ECOUNTER_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x7_position_counts_ECOUNTER_UNDERFLOW"></a>

try_sub would underflow.


<pre><code><b>const</b> <a href="position_counts.md#0x7_position_counts_ECOUNTER_UNDERFLOW">ECOUNTER_UNDERFLOW</a>: u64 = 6;
</code></pre>



<a id="0x7_position_counts_EPOSITION_LIMIT"></a>

try_add would exceed the configured ceiling.


<pre><code><b>const</b> <a href="position_counts.md#0x7_position_counts_EPOSITION_LIMIT">EPOSITION_LIMIT</a>: u64 = 5;
</code></pre>



<a id="0x7_position_counts_init_module"></a>

## Function `init_module`

Runs once when the module is published at <code>@aptos_experimental</code>.


<pre><code><b>fun</b> <a href="position_counts.md#0x7_position_counts_init_module">init_module</a>(experimental: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="position_counts.md#0x7_position_counts_init_module">init_module</a>(experimental: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>if</b> (!<b>exists</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(experimental))) {
        <b>move_to</b>(experimental, <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> { counts: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>() });
    };
}
</code></pre>



</details>

<a id="0x7_position_counts_initialize_for_genesis"></a>

## Function `initialize_for_genesis`

Genesis hook: vm-genesis calls this explicitly after publishing
the framework since it doesn't auto-invoke <code>init_module</code> for
release bundle packages. Called with a signer for
<code>@aptos_experimental</code> (0x7). Idempotent.


<pre><code><b>public</b> <b>fun</b> <a href="position_counts.md#0x7_position_counts_initialize_for_genesis">initialize_for_genesis</a>(experimental: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="position_counts.md#0x7_position_counts_initialize_for_genesis">initialize_for_genesis</a>(experimental: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(experimental) == @aptos_experimental,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="position_counts.md#0x7_position_counts_ENOT_INITIALIZED">ENOT_INITIALIZED</a>),
    );
    <b>if</b> (!<b>exists</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(experimental))) {
        <b>move_to</b>(experimental, <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> { counts: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>() });
    };
}
</code></pre>



</details>

<a id="0x7_position_counts_allocate_counter"></a>

## Function `allocate_counter`

Allocate a counter for a newly registered exchange.

Called by <code><a href="native_position.md#0x7_native_position_register">native_position::register</a>()</code> when the exchange's signer
first registers. If a counter already exists for <code>exchange_id</code>, this
aborts — callers must check for existence first when implementing
idempotent register semantics.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_allocate_counter">allocate_counter</a>(exchange_id: u32, initial_max: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_allocate_counter">allocate_counter</a>(exchange_id: u32, initial_max: u64) <b>acquires</b> <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> {
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(@aptos_experimental),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="position_counts.md#0x7_position_counts_ENOT_INITIALIZED">ENOT_INITIALIZED</a>),
    );
    <b>let</b> counters = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(@aptos_experimental).counts;
    <b>assert</b>!(
        !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(counters, exchange_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="position_counts.md#0x7_position_counts_ECOUNTER_ALREADY_ALLOCATED">ECOUNTER_ALREADY_ALLOCATED</a>),
    );
    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(
        counters,
        exchange_id,
        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(initial_max),
    );
}
</code></pre>



</details>

<a id="0x7_position_counts_counter_exists"></a>

## Function `counter_exists`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_counter_exists">counter_exists</a>(exchange_id: u32): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_counter_exists">counter_exists</a>(exchange_id: u32): bool <b>acquires</b> <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(@aptos_experimental)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> counters = &<b>borrow_global</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(@aptos_experimental).counts;
    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(counters, exchange_id)
}
</code></pre>



</details>

<a id="0x7_position_counts_try_add"></a>

## Function `try_add`

Try to increment the counter for <code>exchange_id</code>. Aborts
<code><a href="position_counts.md#0x7_position_counts_EPOSITION_LIMIT">EPOSITION_LIMIT</a></code> if it would exceed the configured ceiling.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_try_add">try_add</a>(exchange_id: u32, delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_try_add">try_add</a>(exchange_id: u32, delta: u64) <b>acquires</b> <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> {
    <b>let</b> counters = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(@aptos_experimental).counts;
    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(counters, exchange_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="position_counts.md#0x7_position_counts_ECOUNTER_NOT_FOUND">ECOUNTER_NOT_FOUND</a>),
    );
    <b>let</b> agg = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(counters, exchange_id);
    <b>assert</b>!(
        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_try_add">aggregator_v2::try_add</a>(agg, delta),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="position_counts.md#0x7_position_counts_EPOSITION_LIMIT">EPOSITION_LIMIT</a>),
    );
}
</code></pre>



</details>

<a id="0x7_position_counts_sub"></a>

## Function `sub`

Decrement the counter for <code>exchange_id</code>. Aborts <code><a href="position_counts.md#0x7_position_counts_ECOUNTER_UNDERFLOW">ECOUNTER_UNDERFLOW</a></code>
if the counter is already zero.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_sub">sub</a>(exchange_id: u32, delta: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="position_counts.md#0x7_position_counts_sub">sub</a>(exchange_id: u32, delta: u64) <b>acquires</b> <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> {
    <b>let</b> counters = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(@aptos_experimental).counts;
    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(counters, exchange_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="position_counts.md#0x7_position_counts_ECOUNTER_NOT_FOUND">ECOUNTER_NOT_FOUND</a>),
    );
    <b>let</b> agg = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(counters, exchange_id);
    <b>assert</b>!(
        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_try_sub">aggregator_v2::try_sub</a>(agg, delta),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_out_of_range">error::out_of_range</a>(<a href="position_counts.md#0x7_position_counts_ECOUNTER_UNDERFLOW">ECOUNTER_UNDERFLOW</a>),
    );
}
</code></pre>



</details>

<a id="0x7_position_counts_update_ceiling"></a>

## Function `update_ceiling`

Governance-only: adjust the ceiling for an existing exchange's
counter. Typical use: raise before a large migration, lower to
squeeze a misbehaving tenant. Replaces the old aggregator with a
fresh one bounded at <code>new_max</code> and carrying the current value
(clamped to <code>new_max</code> if it would overflow).


<pre><code><b>public</b> <b>fun</b> <a href="position_counts.md#0x7_position_counts_update_ceiling">update_ceiling</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32, new_max: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="position_counts.md#0x7_position_counts_update_ceiling">update_ceiling</a>(
    framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    exchange_id: u32,
    new_max: u64,
) <b>acquires</b> <a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>let</b> counters = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="position_counts.md#0x7_position_counts_PositionCounters">PositionCounters</a>&gt;(@aptos_experimental).counts;
    <b>assert</b>!(
        <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(counters, exchange_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="position_counts.md#0x7_position_counts_ECOUNTER_NOT_FOUND">ECOUNTER_NOT_FOUND</a>),
    );
    <b>let</b> old_agg = <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_remove">table::remove</a>(counters, exchange_id);
    <b>let</b> current = <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_read">aggregator_v2::read</a>(&old_agg);
    <b>let</b> target = <b>if</b> (current &gt; new_max) { new_max } <b>else</b> { current };
    <b>let</b> replacement = <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_create_aggregator">aggregator_v2::create_aggregator</a>(new_max);
    <b>if</b> (target &gt; 0) {
        // Succeeds because target &lt;= new_max.
        <a href="../../aptos-framework/doc/aggregator_v2.md#0x1_aggregator_v2_try_add">aggregator_v2::try_add</a>(&<b>mut</b> replacement, target);
    };
    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(counters, exchange_id, replacement);
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
