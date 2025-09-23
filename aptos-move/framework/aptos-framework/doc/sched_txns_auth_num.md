
<a id="0x1_sched_txns_auth_num"></a>

# Module `0x1::sched_txns_auth_num`



-  [Resource `AuthNumData`](#0x1_sched_txns_auth_num_AuthNumData)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_sched_txns_auth_num_initialize)
-  [Function `get_or_init_auth_num`](#0x1_sched_txns_auth_num_get_or_init_auth_num)
-  [Function `get_auth_num`](#0x1_sched_txns_auth_num_get_auth_num)
-  [Function `increment_auth_num`](#0x1_sched_txns_auth_num_increment_auth_num)
-  [Function `handle_key_rotation`](#0x1_sched_txns_auth_num_handle_key_rotation)
-  [Function `set_auth_num`](#0x1_sched_txns_auth_num_set_auth_num)
-  [Function `contains_addr`](#0x1_sched_txns_auth_num_contains_addr)


<pre><code><b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_sched_txns_auth_num_AuthNumData"></a>

## Resource `AuthNumData`

Stores the authorization number mapping per address


<pre><code><b>struct</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>auth_num_map: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<b>address</b>, u64&gt;</code>
</dt>
<dd>
 BigOrderedMap to track address -> current authorization number
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_sched_txns_auth_num_EAUTH_NUM_NOT_FOUND"></a>

Authorization number not found - must be initialized first via get_auth_num


<pre><code><b>const</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_EAUTH_NUM_NOT_FOUND">EAUTH_NUM_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_sched_txns_auth_num_EINVALID_SIGNER"></a>

Invalid signer - only framework can call this


<pre><code><b>const</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_EINVALID_SIGNER">EINVALID_SIGNER</a>: u64 = 2;
</code></pre>



<a id="0x1_sched_txns_auth_num_initialize"></a>

## Function `initialize`

Initialize the authorization number map - called from scheduled_txns::initialize


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);

    <b>move_to</b>(
        framework,
        <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> { auth_num_map: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_reusable">big_ordered_map::new_with_reusable</a>() }
    );
}
</code></pre>



</details>

<a id="0x1_sched_txns_auth_num_get_or_init_auth_num"></a>

## Function `get_or_init_auth_num`

Returns the current authorization number for an address
Lazy initialization: starts from 1 and stores in map upon first use


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_get_or_init_auth_num">get_or_init_auth_num</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_get_or_init_auth_num">get_or_init_auth_num</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> {
    <b>let</b> data = <b>borrow_global_mut</b>&lt;<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a>&gt;(@aptos_framework);
    <b>if</b> (data.auth_num_map.contains(&addr)) {
        *data.auth_num_map.borrow(&addr)
    } <b>else</b> {
        // Lazy initialization: start from 1
        <b>let</b> initial_auth_num = 1;
        data.auth_num_map.add(addr, initial_auth_num);
        initial_auth_num
    }
}
</code></pre>



</details>

<a id="0x1_sched_txns_auth_num_get_auth_num"></a>

## Function `get_auth_num`

Returns the current authorization number for an address (read-only)
Requires that the address already exists in auth_num_map (initialized via get_or_init_auth_num)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_get_auth_num">get_auth_num</a>(addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_get_auth_num">get_auth_num</a>(addr: <b>address</b>): u64 <b>acquires</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> {
    <b>let</b> data = <b>borrow_global</b>&lt;<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        data.auth_num_map.contains(&addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_EAUTH_NUM_NOT_FOUND">EAUTH_NUM_NOT_FOUND</a>)
    );
    *data.auth_num_map.borrow(&addr)
}
</code></pre>



</details>

<a id="0x1_sched_txns_auth_num_increment_auth_num"></a>

## Function `increment_auth_num`

Increments the authorization number for an address
Requires that the address already exists in auth_num_map (initialized via get_or_init_auth_num)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_increment_auth_num">increment_auth_num</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_increment_auth_num">increment_auth_num</a>(addr: <b>address</b>) <b>acquires</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> {
    <b>let</b> data = <b>borrow_global_mut</b>&lt;<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a>&gt;(@aptos_framework);

    <b>assert</b>!(
        data.auth_num_map.contains(&addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_EAUTH_NUM_NOT_FOUND">EAUTH_NUM_NOT_FOUND</a>)
    );

    <b>let</b> current_auth_num = *data.auth_num_map.borrow(&addr);
    <b>let</b> new_auth_num = current_auth_num + 1;
    *data.auth_num_map.borrow_mut(&addr) = new_auth_num;
}
</code></pre>



</details>

<a id="0x1_sched_txns_auth_num_handle_key_rotation"></a>

## Function `handle_key_rotation`

Handles key rotation by incrementing the authorization number
Only increments if the address already exists in the auth_num_map


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_handle_key_rotation">handle_key_rotation</a>(addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_handle_key_rotation">handle_key_rotation</a>(addr: <b>address</b>) <b>acquires</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> {
    <b>if</b> (<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_contains_addr">contains_addr</a>(addr)) {
        <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_increment_auth_num">increment_auth_num</a>(addr);
    }
    // If sender doesn't exist, do nothing
}
</code></pre>



</details>

<a id="0x1_sched_txns_auth_num_set_auth_num"></a>

## Function `set_auth_num`

Sets a specific authorization number for an address (useful for testing or migration)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_set_auth_num">set_auth_num</a>(addr: <b>address</b>, auth_num: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_set_auth_num">set_auth_num</a>(addr: <b>address</b>, auth_num: u64) <b>acquires</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> {
    <b>let</b> data = <b>borrow_global_mut</b>&lt;<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a>&gt;(@aptos_framework);
    <b>if</b> (data.auth_num_map.contains(&addr)) {
        *data.auth_num_map.borrow_mut(&addr) = auth_num;
    } <b>else</b> {
        data.auth_num_map.add(addr, auth_num);
    }
}
</code></pre>



</details>

<a id="0x1_sched_txns_auth_num_contains_addr"></a>

## Function `contains_addr`

Checks if an address exists in the authorization number map


<pre><code><b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_contains_addr">contains_addr</a>(addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_contains_addr">contains_addr</a>(addr: <b>address</b>): bool <b>acquires</b> <a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a> {
    <b>let</b> data = <b>borrow_global</b>&lt;<a href="sched_txns_auth_num.md#0x1_sched_txns_auth_num_AuthNumData">AuthNumData</a>&gt;(@aptos_framework);
    data.auth_num_map.contains(&addr)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
