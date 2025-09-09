
<a id="0x1_sched_txns_sender_seqno"></a>

# Module `0x1::sched_txns_sender_seqno`



-  [Resource `SenderSeqnoData`](#0x1_sched_txns_sender_seqno_SenderSeqnoData)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_sched_txns_sender_seqno_initialize)
-  [Function `get_sender_seqno`](#0x1_sched_txns_sender_seqno_get_sender_seqno)
-  [Function `get_sender_seqno_readonly`](#0x1_sched_txns_sender_seqno_get_sender_seqno_readonly)
-  [Function `increment_sender_seqno`](#0x1_sched_txns_sender_seqno_increment_sender_seqno)
-  [Function `handle_key_rotation`](#0x1_sched_txns_sender_seqno_handle_key_rotation)
-  [Function `destroy_sender_seqno_map`](#0x1_sched_txns_sender_seqno_destroy_sender_seqno_map)
-  [Function `set_sender_seqno`](#0x1_sched_txns_sender_seqno_set_sender_seqno)
-  [Function `contains_sender`](#0x1_sched_txns_sender_seqno_contains_sender)


<pre><code><b>use</b> <a href="big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_sched_txns_sender_seqno_SenderSeqnoData"></a>

## Resource `SenderSeqnoData`

Stores the sender sequence number mapping


<pre><code><b>struct</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>sender_seqno_map: <a href="big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<b>address</b>, u64&gt;</code>
</dt>
<dd>
 BigOrderedMap to track sender address -> current sequence number for authorization
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_sched_txns_sender_seqno_EINVALID_SIGNER"></a>

Invalid signer - only framework can call this


<pre><code><b>const</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_EINVALID_SIGNER">EINVALID_SIGNER</a>: u64 = 2;
</code></pre>



<a id="0x1_sched_txns_sender_seqno_ESENDER_SEQNO_NOT_FOUND"></a>

Sender sequence number not found - must be initialized first via get_sender_seqno


<pre><code><b>const</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_ESENDER_SEQNO_NOT_FOUND">ESENDER_SEQNO_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_sched_txns_sender_seqno_initialize"></a>

## Function `initialize`

Initialize the sender sequence number map - called from scheduled_txns::initialize


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);

    <b>move_to</b>(
        framework,
        <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> { sender_seqno_map: <a href="big_ordered_map.md#0x1_big_ordered_map_new_with_reusable">big_ordered_map::new_with_reusable</a>() }
    );
}
</code></pre>



</details>

<a id="0x1_sched_txns_sender_seqno_get_sender_seqno"></a>

## Function `get_sender_seqno`

Returns the current authorization sequence number for a sender address
Lazy initialization: starts from 1 and stores in map upon first use


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_get_sender_seqno">get_sender_seqno</a>(sender_addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_get_sender_seqno">get_sender_seqno</a>(sender_addr: <b>address</b>): u64 <b>acquires</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> {
    <b>let</b> seqno_data = <b>borrow_global_mut</b>&lt;<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a>&gt;(@aptos_framework);
    <b>if</b> (seqno_data.sender_seqno_map.contains(&sender_addr)) {
        *seqno_data.sender_seqno_map.borrow(&sender_addr)
    } <b>else</b> {
        // Lazy initialization: start from 1
        <b>let</b> initial_seqno = 1;
        seqno_data.sender_seqno_map.add(sender_addr, initial_seqno);
        initial_seqno
    }
}
</code></pre>



</details>

<a id="0x1_sched_txns_sender_seqno_get_sender_seqno_readonly"></a>

## Function `get_sender_seqno_readonly`

Returns the current authorization sequence number for a sender address (read-only)
Requires that the sender already exists in sender_seqno_map (initialized via get_sender_seqno)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_get_sender_seqno_readonly">get_sender_seqno_readonly</a>(sender_addr: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_get_sender_seqno_readonly">get_sender_seqno_readonly</a>(sender_addr: <b>address</b>): u64 <b>acquires</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> {
    <b>let</b> seqno_data = <b>borrow_global</b>&lt;<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        seqno_data.sender_seqno_map.contains(&sender_addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_ESENDER_SEQNO_NOT_FOUND">ESENDER_SEQNO_NOT_FOUND</a>)
    );
    *seqno_data.sender_seqno_map.borrow(&sender_addr)
}
</code></pre>



</details>

<a id="0x1_sched_txns_sender_seqno_increment_sender_seqno"></a>

## Function `increment_sender_seqno`

Increments the sequence number for a sender address
Requires that the sender already exists in sender_seqno_map (initialized via get_sender_seqno)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_increment_sender_seqno">increment_sender_seqno</a>(sender_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_increment_sender_seqno">increment_sender_seqno</a>(sender_addr: <b>address</b>) <b>acquires</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> {
    <b>let</b> seqno_data = <b>borrow_global_mut</b>&lt;<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a>&gt;(@aptos_framework);

    // Assert that sender <b>exists</b> in map - must be initialized first via get_sender_seqno
    <b>assert</b>!(
        seqno_data.sender_seqno_map.contains(&sender_addr),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_ESENDER_SEQNO_NOT_FOUND">ESENDER_SEQNO_NOT_FOUND</a>)
    );

    <b>let</b> current_seqno = *seqno_data.sender_seqno_map.borrow(&sender_addr);
    <b>let</b> new_seqno = current_seqno + 1;
    *seqno_data.sender_seqno_map.borrow_mut(&sender_addr) = new_seqno;
}
</code></pre>



</details>

<a id="0x1_sched_txns_sender_seqno_handle_key_rotation"></a>

## Function `handle_key_rotation`

Handles key rotation by incrementing the sender sequence number
Only increments if the sender already exists in the sender_seqno_map


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_handle_key_rotation">handle_key_rotation</a>(sender_addr: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_handle_key_rotation">handle_key_rotation</a>(sender_addr: <b>address</b>) <b>acquires</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> {
    <b>if</b> (<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_contains_sender">contains_sender</a>(sender_addr)) {
        <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_increment_sender_seqno">increment_sender_seqno</a>(sender_addr);
    }
    // If sender doesn't exist, do nothing
}
</code></pre>



</details>

<a id="0x1_sched_txns_sender_seqno_destroy_sender_seqno_map"></a>

## Function `destroy_sender_seqno_map`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_destroy_sender_seqno_map">destroy_sender_seqno_map</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_destroy_sender_seqno_map">destroy_sender_seqno_map</a>() <b>acquires</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> {
    <b>let</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> { sender_seqno_map } =
        <b>move_from</b>&lt;<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a>&gt;(@aptos_framework);
    // Clear all elements from the map before dropping it
    sender_seqno_map.for_each(
        |_key, _value| {
            // Do nothing - just consume the elements
        }
    );
}
</code></pre>



</details>

<a id="0x1_sched_txns_sender_seqno_set_sender_seqno"></a>

## Function `set_sender_seqno`

Sets a specific sequence number for a sender (useful for testing or migration)


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_set_sender_seqno">set_sender_seqno</a>(sender_addr: <b>address</b>, seqno: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_set_sender_seqno">set_sender_seqno</a>(sender_addr: <b>address</b>, seqno: u64) <b>acquires</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> {
    <b>let</b> seqno_data = <b>borrow_global_mut</b>&lt;<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a>&gt;(@aptos_framework);
    <b>if</b> (seqno_data.sender_seqno_map.contains(&sender_addr)) {
        *seqno_data.sender_seqno_map.borrow_mut(&sender_addr) = seqno;
    } <b>else</b> {
        seqno_data.sender_seqno_map.add(sender_addr, seqno);
    }
}
</code></pre>



</details>

<a id="0x1_sched_txns_sender_seqno_contains_sender"></a>

## Function `contains_sender`

Checks if a sender exists in the sequence number map


<pre><code><b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_contains_sender">contains_sender</a>(sender_addr: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_contains_sender">contains_sender</a>(sender_addr: <b>address</b>): bool <b>acquires</b> <a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a> {
    <b>let</b> seqno_data = <b>borrow_global</b>&lt;<a href="sched_txns_sender_seqno.md#0x1_sched_txns_sender_seqno_SenderSeqnoData">SenderSeqnoData</a>&gt;(@aptos_framework);
    seqno_data.sender_seqno_map.contains(&sender_addr)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
