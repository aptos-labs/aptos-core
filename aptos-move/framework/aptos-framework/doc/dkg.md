
<a name="0x1_dkg"></a>

# Module `0x1::dkg`



-  [Struct `StartDKGEvent`](#0x1_dkg_StartDKGEvent)
-  [Struct `DKGSessionState`](#0x1_dkg_DKGSessionState)
-  [Resource `DKGState`](#0x1_dkg_DKGState)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_dkg_initialize)
-  [Function `session_in_progress`](#0x1_dkg_session_in_progress)
-  [Function `start`](#0x1_dkg_start)
-  [Function `finish`](#0x1_dkg_finish)


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
<code>target_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_dkg_DKGSessionState"></a>

## Struct `DKGSessionState`

The input and output of a DKG session.
The validator set of epoch <code>x</code> works together and outputs a transcript for the target validator set of epoch <code>y</code> (typically <code>x+1</code>).


<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dealer_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>dealer_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a></code>
</dt>
<dd>

</dd>
<dt>
<code>target_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>target_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a></code>
</dt>
<dd>

</dd>
<dt>
<code>serialized_transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_dkg_DKGState"></a>

## Resource `DKGState`

The complete and ongoing DKG sessions.


<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>last_complete: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>in_progress: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>&gt;</code>
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
            last_complete: std::option::none(),
            in_progress: std::option::none(),
            events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a>&gt;(aptos_framework),
        }
    );
}
</code></pre>



</details>

<a name="0x1_dkg_session_in_progress"></a>

## Function `session_in_progress`

Return the currently in-progress DKG session, if there is one.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_session_in_progress">session_in_progress</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_session_in_progress">session_in_progress</a>(): Option&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a>&gt; <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <b>let</b> dkg_state = <b>borrow_global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    dkg_state.in_progress
}
</code></pre>



</details>

<a name="0x1_dkg_start"></a>

## Function `start`

Mark the start of a new DKG session by storing the input. Also emit the <code><a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a></code>.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(dealer_epoch: u64, dealer_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>, target_epoch: u64, target_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(dealer_epoch: u64, dealer_validator_set: ValidatorSet, target_epoch: u64, target_validator_set: ValidatorSet) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"<a href="dkg.md#0x1_dkg_start">dkg::start</a>() - Started."));
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <b>assert</b>!(std::option::is_none(&dkg_state.in_progress), 1);
    dkg_state.in_progress = std::option::some(<a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a> {
        dealer_epoch,
        dealer_validator_set,
        target_epoch,
        target_validator_set,
        serialized_transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    });
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a>&gt;(
        &<b>mut</b> dkg_state.events,
        <a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a> {
            target_epoch,
            target_validator_set,
        },
    );
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"<a href="dkg.md#0x1_dkg_start">dkg::start</a>() - Finished."))
}
</code></pre>



</details>

<a name="0x1_dkg_finish"></a>

## Function `finish`

Mark the ongoing DKG session complete.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_finish">finish</a>(serialized_transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_finish">finish</a>(serialized_transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_finish">dkg::finish</a>() - Started."));
    <b>let</b> session_in_progress = std::option::extract(&<b>mut</b> <a href="dkg.md#0x1_dkg_session_in_progress">session_in_progress</a>());
    session_in_progress.serialized_transcript = serialized_transcript;
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    state.last_complete = std::option::some(session_in_progress);
    state.in_progress = std::option::none();
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&std::string::utf8(b"<a href="dkg.md#0x1_dkg_finish">dkg::finish</a>() - Finished."))
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
