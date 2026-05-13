
<a id="0x7_migration_complete"></a>

# Module `0x7::migration_complete`

Framework-level marker that governance sets after a per-exchange
off-chain migration sweep confirms no stale <code>UserPositions</code>
resources remain. Applications (e.g. etna) check for the marker's
existence at their Phase 4 cutover as described in
<code>PLAN_native_position.md</code>.

Semantics:
- One <code>MigrationComplete</code> resource per <code>exchange_id</code>, stored at
<code>@aptos_experimental</code>.
- Only <code>aptos_framework</code> can call <code>finalize</code> — this is a
governance action paired with the off-chain sweep.
- Once set, the marker is monotonic: cannot be cleared. Rolling
back a cutover requires application-side reverse-migration,
not marker removal.


-  [Struct `CompletionEntry`](#0x7_migration_complete_CompletionEntry)
-  [Resource `MigrationCompleteRegistry`](#0x7_migration_complete_MigrationCompleteRegistry)
-  [Constants](#@Constants_0)
-  [Function `init_module`](#0x7_migration_complete_init_module)
-  [Function `finalize`](#0x7_migration_complete_finalize)
-  [Function `is_finalized`](#0x7_migration_complete_is_finalized)
-  [Function `finalized_at`](#0x7_migration_complete_finalized_at)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x7_migration_complete_CompletionEntry"></a>

## Struct `CompletionEntry`



<pre><code><b>struct</b> <a href="migration_complete.md#0x7_migration_complete_CompletionEntry">CompletionEntry</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>finalized_at_version: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_migration_complete_MigrationCompleteRegistry"></a>

## Resource `MigrationCompleteRegistry`



<pre><code><b>struct</b> <a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>entries: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u32, <a href="migration_complete.md#0x7_migration_complete_CompletionEntry">migration_complete::CompletionEntry</a>&gt;</code>
</dt>
<dd>
 exchange_id -> finalization entry
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x7_migration_complete_EALREADY_FINALIZED"></a>

<code>finalize</code> was called more than once for the same <code>exchange_id</code>.


<pre><code><b>const</b> <a href="migration_complete.md#0x7_migration_complete_EALREADY_FINALIZED">EALREADY_FINALIZED</a>: u64 = 2;
</code></pre>



<a id="0x7_migration_complete_ENOT_INITIALIZED"></a>

The framework-level <code>MigrationComplete</code> registry has not been
initialized yet.


<pre><code><b>const</b> <a href="migration_complete.md#0x7_migration_complete_ENOT_INITIALIZED">ENOT_INITIALIZED</a>: u64 = 1;
</code></pre>



<a id="0x7_migration_complete_init_module"></a>

## Function `init_module`

Runs once when the module is published at <code>@aptos_experimental</code>.


<pre><code><b>fun</b> <a href="migration_complete.md#0x7_migration_complete_init_module">init_module</a>(experimental: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="migration_complete.md#0x7_migration_complete_init_module">init_module</a>(experimental: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(
        experimental,
        <a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a> { entries: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>() },
    );
}
</code></pre>



</details>

<a id="0x7_migration_complete_finalize"></a>

## Function `finalize`

Governance-only: mark migration complete for <code>exchange_id</code>.
<code>finalized_at_version</code> should be the version at which the
off-chain sweep last observed zero remaining <code>UserPositions</code>.


<pre><code><b>public</b> <b>fun</b> <a href="migration_complete.md#0x7_migration_complete_finalize">finalize</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, exchange_id: u32, finalized_at_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="migration_complete.md#0x7_migration_complete_finalize">finalize</a>(
    framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    exchange_id: u32,
    finalized_at_version: u64,
) <b>acquires</b> <a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a> {
    <a href="../../aptos-framework/doc/system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>assert</b>!(
        <b>exists</b>&lt;<a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a>&gt;(@aptos_experimental),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="migration_complete.md#0x7_migration_complete_ENOT_INITIALIZED">ENOT_INITIALIZED</a>),
    );
    <b>let</b> registry =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a>&gt;(@aptos_experimental).entries;
    <b>assert</b>!(
        !<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(registry, exchange_id),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="migration_complete.md#0x7_migration_complete_EALREADY_FINALIZED">EALREADY_FINALIZED</a>),
    );
    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(
        registry,
        exchange_id,
        <a href="migration_complete.md#0x7_migration_complete_CompletionEntry">CompletionEntry</a> { finalized_at_version },
    );
}
</code></pre>



</details>

<a id="0x7_migration_complete_is_finalized"></a>

## Function `is_finalized`

True if <code>exchange_id</code> has had its migration finalized by
governance. Application-side code calls this before deploying
or executing the no-legacy-path module version.


<pre><code><b>public</b> <b>fun</b> <a href="migration_complete.md#0x7_migration_complete_is_finalized">is_finalized</a>(exchange_id: u32): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="migration_complete.md#0x7_migration_complete_is_finalized">is_finalized</a>(exchange_id: u32): bool <b>acquires</b> <a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a>&gt;(@aptos_experimental)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> entries = &<b>borrow_global</b>&lt;<a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a>&gt;(@aptos_experimental).entries;
    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(entries, exchange_id)
}
</code></pre>



</details>

<a id="0x7_migration_complete_finalized_at"></a>

## Function `finalized_at`

Return the version at which migration was finalized, or 0 if
not yet finalized.


<pre><code><b>public</b> <b>fun</b> <a href="migration_complete.md#0x7_migration_complete_finalized_at">finalized_at</a>(exchange_id: u32): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="migration_complete.md#0x7_migration_complete_finalized_at">finalized_at</a>(exchange_id: u32): u64 <b>acquires</b> <a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a>&gt;(@aptos_experimental)) {
        <b>return</b> 0
    };
    <b>let</b> entries = &<b>borrow_global</b>&lt;<a href="migration_complete.md#0x7_migration_complete_MigrationCompleteRegistry">MigrationCompleteRegistry</a>&gt;(@aptos_experimental).entries;
    <b>if</b> (!<a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(entries, exchange_id)) {
        <b>return</b> 0
    };
    <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(entries, exchange_id).finalized_at_version
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
