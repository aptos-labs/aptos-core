
<a id="0x1_delegation_pool_allowlist"></a>

# Module `0x1::delegation_pool_allowlist`

This module implements a detached allowlist of delegators that are accepted into one's delegation pool.
Any account can edit their owned allowlist, but a delegation pool will only use the allowlist defined
under its owner's account.


-  [Resource `DelegationPoolAllowlisting`](#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting)
-  [Struct `EnableDelegatorsAllowlisting`](#0x1_delegation_pool_allowlist_EnableDelegatorsAllowlisting)
-  [Struct `DisableDelegatorsAllowlisting`](#0x1_delegation_pool_allowlist_DisableDelegatorsAllowlisting)
-  [Struct `AllowlistDelegator`](#0x1_delegation_pool_allowlist_AllowlistDelegator)
-  [Struct `RemoveDelegatorFromAllowlist`](#0x1_delegation_pool_allowlist_RemoveDelegatorFromAllowlist)
-  [Constants](#@Constants_0)
-  [Function `allowlisting_enabled`](#0x1_delegation_pool_allowlist_allowlisting_enabled)
-  [Function `delegator_allowlisted`](#0x1_delegation_pool_allowlist_delegator_allowlisted)
-  [Function `get_delegators_allowlist`](#0x1_delegation_pool_allowlist_get_delegators_allowlist)
-  [Function `enable_delegators_allowlisting`](#0x1_delegation_pool_allowlist_enable_delegators_allowlisting)
-  [Function `disable_delegators_allowlisting`](#0x1_delegation_pool_allowlist_disable_delegators_allowlisting)
-  [Function `allowlist_delegator`](#0x1_delegation_pool_allowlist_allowlist_delegator)
-  [Function `remove_delegator_from_allowlist`](#0x1_delegation_pool_allowlist_remove_delegator_from_allowlist)
-  [Function `assert_allowlisting_enabled`](#0x1_delegation_pool_allowlist_assert_allowlisting_enabled)
-  [Function `borrow_mut_delegators_allowlist`](#0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table">0x1::smart_table</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table_with_length.md#0x1_table_with_length">0x1::table_with_length</a>;
</code></pre>



<a id="0x1_delegation_pool_allowlist_DelegationPoolAllowlisting"></a>

## Resource `DelegationPoolAllowlisting`

Tracks a delegation pool's allowlist of delegators.
A delegation pool will only use the allowlist defined under its owner's account.
If allowlisting is enabled, existing delegators are not implicitly allowlisted and they can be individually
evicted later by the pool owner.


<pre><code><b>struct</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> <b>has</b> store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>allowlist: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<b>address</b>, bool&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_allowlist_EnableDelegatorsAllowlisting"></a>

## Struct `EnableDelegatorsAllowlisting`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_EnableDelegatorsAllowlisting">EnableDelegatorsAllowlisting</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_allowlist_DisableDelegatorsAllowlisting"></a>

## Struct `DisableDelegatorsAllowlisting`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DisableDelegatorsAllowlisting">DisableDelegatorsAllowlisting</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_allowlist_AllowlistDelegator"></a>

## Struct `AllowlistDelegator`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_AllowlistDelegator">AllowlistDelegator</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_delegation_pool_allowlist_RemoveDelegatorFromAllowlist"></a>

## Struct `RemoveDelegatorFromAllowlist`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_RemoveDelegatorFromAllowlist">RemoveDelegatorFromAllowlist</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>owner_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>delegator_address: <b>address</b></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_delegation_pool_allowlist_EDELEGATORS_ALLOWLISTING_NOT_ENABLED"></a>

Delegators allowlisting should be enabled to perform this operation.


<pre><code><b>const</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_EDELEGATORS_ALLOWLISTING_NOT_ENABLED">EDELEGATORS_ALLOWLISTING_NOT_ENABLED</a>: u64 = 2;
</code></pre>



<a id="0x1_delegation_pool_allowlist_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED"></a>

Delegators allowlisting is not supported.


<pre><code><b>const</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED">EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED</a>: u64 = 1;
</code></pre>



<a id="0x1_delegation_pool_allowlist_allowlisting_enabled"></a>

## Function `allowlisting_enabled`

Return whether allowlisting is enabled for the provided delegation pool owner.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_allowlisting_enabled">allowlisting_enabled</a>(owner_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_allowlisting_enabled">allowlisting_enabled</a>(owner_address: <b>address</b>): bool {
    <b>exists</b>&lt;<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(owner_address)
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_delegator_allowlisted"></a>

## Function `delegator_allowlisted`

Return whether the provided delegator is allowlisted.
A delegator is allowlisted if:
- allowlisting is disabled on the delegation pool's owner
- delegator is part of the allowlist


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_delegator_allowlisted">delegator_allowlisted</a>(owner_address: <b>address</b>, delegator_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_delegator_allowlisted">delegator_allowlisted</a>(
    owner_address: <b>address</b>,
    delegator_address: <b>address</b>,
): bool <b>acquires</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <b>if</b> (!<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_allowlisting_enabled">allowlisting_enabled</a>(owner_address)) { <b>return</b> <b>true</b> };

    *<a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_borrow_with_default">smart_table::borrow_with_default</a>(
        <b>freeze</b>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(owner_address)),
        delegator_address,
        &<b>false</b>
    )
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_get_delegators_allowlist"></a>

## Function `get_delegators_allowlist`

Return allowlist or revert if allowlisting is not enabled for the provided owner account.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_get_delegators_allowlist">get_delegators_allowlist</a>(owner_address: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_get_delegators_allowlist">get_delegators_allowlist</a>(
    owner_address: <b>address</b>,
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt; <b>acquires</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(owner_address);

    <b>let</b> allowlist = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_for_each_ref">smart_table::for_each_ref</a>(<b>freeze</b>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(owner_address)), |delegator, _included| {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> allowlist, *delegator);
    });
    allowlist
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_enable_delegators_allowlisting"></a>

## Function `enable_delegators_allowlisting`

Enable delegators allowlisting as the pool owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_enable_delegators_allowlisting">enable_delegators_allowlisting</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_enable_delegators_allowlisting">enable_delegators_allowlisting</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_delegation_pool_allowlisting_enabled">features::delegation_pool_allowlisting_enabled</a>(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED">EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED</a>)
    );

    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <b>if</b> (<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_allowlisting_enabled">allowlisting_enabled</a>(owner_address)) { <b>return</b> };

    <b>move_to</b>(owner, <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> { allowlist: <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_new">smart_table::new</a>&lt;<b>address</b>, bool&gt;() });

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_EnableDelegatorsAllowlisting">EnableDelegatorsAllowlisting</a> { owner_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_disable_delegators_allowlisting"></a>

## Function `disable_delegators_allowlisting`

Disable delegators allowlisting as the pool owner. The existing allowlist will be emptied.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_disable_delegators_allowlisting">disable_delegators_allowlisting</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_disable_delegators_allowlisting">disable_delegators_allowlisting</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
) <b>acquires</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(owner_address);

    <b>let</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> { allowlist } = <b>move_from</b>&lt;<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(owner_address);
    // <b>if</b> the allowlist becomes too large, the owner can always remove some delegators
    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_destroy">smart_table::destroy</a>(allowlist);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DisableDelegatorsAllowlisting">DisableDelegatorsAllowlisting</a> { owner_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_allowlist_delegator"></a>

## Function `allowlist_delegator`

Allowlist a delegator as the pool owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_allowlist_delegator">allowlist_delegator</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_allowlist_delegator">allowlist_delegator</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    delegator_address: <b>address</b>,
) <b>acquires</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(owner_address);

    <b>if</b> (<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_delegator_allowlisted">delegator_allowlisted</a>(owner_address, delegator_address)) { <b>return</b> };

    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_add">smart_table::add</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(owner_address), delegator_address, <b>true</b>);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_AllowlistDelegator">AllowlistDelegator</a> { owner_address, delegator_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_remove_delegator_from_allowlist"></a>

## Function `remove_delegator_from_allowlist`

Remove a delegator from the allowlist as the pool owner.


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_remove_delegator_from_allowlist">remove_delegator_from_allowlist</a>(owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, delegator_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_remove_delegator_from_allowlist">remove_delegator_from_allowlist</a>(
    owner: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    delegator_address: <b>address</b>,
) <b>acquires</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    <b>let</b> owner_address = <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);
    <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(owner_address);

    <b>if</b> (!<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_delegator_allowlisted">delegator_allowlisted</a>(owner_address, delegator_address)) { <b>return</b> };

    <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_remove">smart_table::remove</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(owner_address), delegator_address);

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_RemoveDelegatorFromAllowlist">RemoveDelegatorFromAllowlist</a> { owner_address, delegator_address });
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_assert_allowlisting_enabled"></a>

## Function `assert_allowlisting_enabled`



<pre><code><b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(owner_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_assert_allowlisting_enabled">assert_allowlisting_enabled</a>(owner_address: <b>address</b>) {
    <b>assert</b>!(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_allowlisting_enabled">allowlisting_enabled</a>(owner_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_EDELEGATORS_ALLOWLISTING_NOT_ENABLED">EDELEGATORS_ALLOWLISTING_NOT_ENABLED</a>));
}
</code></pre>



</details>

<a id="0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist"></a>

## Function `borrow_mut_delegators_allowlist`



<pre><code><b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(owner_address: <b>address</b>): &<b>mut</b> <a href="../../aptos-stdlib/doc/smart_table.md#0x1_smart_table_SmartTable">smart_table::SmartTable</a>&lt;<b>address</b>, bool&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_borrow_mut_delegators_allowlist">borrow_mut_delegators_allowlist</a>(
    owner_address: <b>address</b>
): &<b>mut</b> SmartTable&lt;<b>address</b>, bool&gt; <b>acquires</b> <a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a> {
    &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="delegation_pool_allowlist.md#0x1_delegation_pool_allowlist_DelegationPoolAllowlisting">DelegationPoolAllowlisting</a>&gt;(owner_address).allowlist
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
