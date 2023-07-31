
<a name="0x1_dkg"></a>

# Module `0x1::dkg`



-  [Struct `StartDKGEvent`](#0x1_dkg_StartDKGEvent)
-  [Resource `DKGState`](#0x1_dkg_DKGState)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_dkg_initialize)
-  [Function `get_state`](#0x1_dkg_get_state)
-  [Function `state_started`](#0x1_dkg_state_started)
-  [Function `state_not_started`](#0x1_dkg_state_not_started)
-  [Function `start`](#0x1_dkg_start)
-  [Function `finish`](#0x1_dkg_finish)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a name="0x1_dkg_StartDKGEvent"></a>

## Struct `StartDKGEvent`



<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>validator_set_and_stake_dist: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_dkg_DKGState"></a>

## Resource `DKGState`



<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>state_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">dkg::StartDKGEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_dkg_EINVALID_GUID_FOR_EVENT"></a>

An invalid block time was encountered.


<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_EINVALID_GUID_FOR_EVENT">EINVALID_GUID_FOR_EVENT</a>: u64 = 5;
</code></pre>



<a name="0x1_dkg_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>assert</b>!(5 == <a href="account.md#0x1_account_get_guid_next_creation_num">account::get_guid_next_creation_num</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework)), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="dkg.md#0x1_dkg_EINVALID_GUID_FOR_EVENT">EINVALID_GUID_FOR_EVENT</a>));
    <b>move_to</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(
        aptos_framework,
        <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
            state_id: 0,
            events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a>&gt;(aptos_framework),
        }
    );
}
</code></pre>



</details>

<a name="0x1_dkg_get_state"></a>

## Function `get_state`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_get_state">get_state</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> (<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_get_state">get_state</a>(): u64 <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a>  {
    <b>let</b> dkg_state = <b>borrow_global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    dkg_state.state_id
}
</code></pre>



</details>

<a name="0x1_dkg_state_started"></a>

## Function `state_started`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_started">state_started</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> (<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_started">state_started</a>(): u64 {
    1
}
</code></pre>



</details>

<a name="0x1_dkg_state_not_started"></a>

## Function `state_not_started`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_not_started">state_not_started</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> (<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_not_started">state_not_started</a>(): u64 {
    0
}
</code></pre>



</details>

<a name="0x1_dkg_start"></a>

## Function `start`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(validator_set_and_stake_dist: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(validator_set_and_stake_dist: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_start">dkg::start</a>() started."));
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <b>if</b> (dkg_state.state_id != 0) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_start">dkg::start</a>() called <b>while</b> <a href="dkg.md#0x1_dkg">dkg</a> already started."));
        <b>return</b>;
    };
    dkg_state.state_id = 1;
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a>&gt;(
        &<b>mut</b> dkg_state.events,
        <a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a> {
            validator_set_and_stake_dist,
        },
    );
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_start">dkg::start</a>() finished."));
}
</code></pre>



</details>

<a name="0x1_dkg_finish"></a>

## Function `finish`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_finish">finish</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_finish">finish</a>() <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_finish">dkg::finish</a>() started."));
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    dkg_state.state_id = 0;
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_finish">dkg::finish</a>() started."));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
