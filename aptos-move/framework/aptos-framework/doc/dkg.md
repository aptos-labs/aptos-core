
<a name="0x1_dkg"></a>

# Module `0x1::dkg`



-  [Struct `StartDKGEvent`](#0x1_dkg_StartDKGEvent)
-  [Resource `DKGState`](#0x1_dkg_DKGState)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_dkg_initialize)
-  [Function `get_state`](#0x1_dkg_get_state)
-  [Function `state_active`](#0x1_dkg_state_active)
-  [Function `state_inactive`](#0x1_dkg_state_inactive)
-  [Function `start`](#0x1_dkg_start)
-  [Function `on_potential_transcript`](#0x1_dkg_on_potential_transcript)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
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
<code>target_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>locked_new_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;</code>
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
<code>target_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>state_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>countdown: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>serialized_transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 DKG Transcript for current epoch.
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
            target_epoch: 1,
            state_id: 0,
            countdown: 0,
            serialized_transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
            events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a>&gt;(aptos_framework),
        }
    );
}
</code></pre>



</details>

<a name="0x1_dkg_get_state"></a>

## Function `get_state`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_get_state">get_state</a>(): (u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> (<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_get_state">get_state</a>(): (u64, u64) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a>  {
    <b>let</b> dkg_state = <b>borrow_global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    (dkg_state.target_epoch, dkg_state.state_id)
}
</code></pre>



</details>

<a name="0x1_dkg_state_active"></a>

## Function `state_active`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_active">state_active</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> (<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_active">state_active</a>(): u64 {
    1
}
</code></pre>



</details>

<a name="0x1_dkg_state_inactive"></a>

## Function `state_inactive`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_inactive">state_inactive</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> (<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_state_inactive">state_inactive</a>(): u64 {
    0
}
</code></pre>



</details>

<a name="0x1_dkg_start"></a>

## Function `start`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(target_epoch: u64, locked_new_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="stake.md#0x1_stake_ValidatorInfo">stake::ValidatorInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(target_epoch: u64, locked_new_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorInfo&gt;) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"<a href="dkg.md#0x1_dkg_start">dkg::start</a>() started."));
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"dkg_state="));
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(dkg_state);
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"target_epoch="));
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&target_epoch);
    <b>if</b> (target_epoch == dkg_state.target_epoch + 1 && dkg_state.state_id == 0) {
        dkg_state.target_epoch = target_epoch;
        dkg_state.state_id = 1;
        dkg_state.countdown = 999999999; //TODO: for debugging
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a>&gt;(
            &<b>mut</b> dkg_state.events,
            <a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a> {
                target_epoch,
                locked_new_validator_set,
            },
        );
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"unexpected <a href="dkg.md#0x1_dkg_start">dkg::start</a>()..."));
    };
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"<a href="dkg.md#0x1_dkg_start">dkg::start</a>() finished."));
}
</code></pre>



</details>

<a name="0x1_dkg_on_potential_transcript"></a>

## Function `on_potential_transcript`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_on_potential_transcript">on_potential_transcript</a>(maybe_serialized_transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_on_potential_transcript">on_potential_transcript</a>(maybe_serialized_transcript: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;): bool <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_on_potential_transcript">dkg::on_potential_transcript</a>() - Started."));
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <b>assert</b>!(<a href="dkg.md#0x1_dkg_state_active">state_active</a>() == dkg_state.state_id, 1);
    <b>let</b> ret = <b>if</b> (std::option::is_some(&maybe_serialized_transcript)) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_on_potential_transcript">dkg::on_potential_transcript</a>() - A transcript is given!"));
        dkg_state.state_id = 0;
        dkg_state.countdown = 0;
        dkg_state.serialized_transcript = std::option::extract(&<b>mut</b> maybe_serialized_transcript);
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&dkg_state.serialized_transcript);
        <b>true</b>
    } <b>else</b> <b>if</b> (dkg_state.countdown == 0) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_on_potential_transcript">dkg::on_potential_transcript</a>() - Current DKG is taking too long. Aborting."));
        dkg_state.state_id = 0;
        dkg_state.serialized_transcript = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
        <b>true</b>
    } <b>else</b> {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_on_potential_transcript">dkg::on_potential_transcript</a>() - No transcript is given. Hopefully next <a href="block.md#0x1_block">block</a>."));
        dkg_state.countdown = dkg_state.countdown - 1;
        <b>false</b>
    };
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_on_potential_transcript">dkg::on_potential_transcript</a>() - Finished."));
    ret
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
