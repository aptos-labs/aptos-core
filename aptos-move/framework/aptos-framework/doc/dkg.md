
<a name="0x1_dkg"></a>

# Module `0x1::dkg`



-  [Struct `StartDKGEvent`](#0x1_dkg_StartDKGEvent)
-  [Struct `DKGSessionState`](#0x1_dkg_DKGSessionState)
-  [Resource `DKGState`](#0x1_dkg_DKGState)
-  [Constants](#@Constants_0)
-  [Function `start`](#0x1_dkg_start)
-  [Function `update`](#0x1_dkg_update)
-  [Function `in_progress`](#0x1_dkg_in_progress)
-  [Function `current_deadline`](#0x1_dkg_current_deadline)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
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
<code>result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>deadline_microseconds: u64</code>
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


<a name="0x1_dkg_EANOTHER_RECONFIGURATION_IN_PROGRESS"></a>

Another reconfiguration is in progress.


<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_EANOTHER_RECONFIGURATION_IN_PROGRESS">EANOTHER_RECONFIGURATION_IN_PROGRESS</a>: u64 = 1;
</code></pre>



<a name="0x1_dkg_ENO_RECONFIGURATION_IN_PROGRESS"></a>

There is no reconfiguration in progress.


<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_ENO_RECONFIGURATION_IN_PROGRESS">ENO_RECONFIGURATION_IN_PROGRESS</a>: u64 = 2;
</code></pre>



<a name="0x1_dkg_start"></a>

## Function `start`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(dealer_epoch: u64, dealer_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>, target_epoch: u64, target_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(dealer_epoch: u64, dealer_validator_set: ValidatorSet, target_epoch: u64, target_validator_set: ValidatorSet) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <b>assert</b>!(std::option::is_none(&dkg_state.in_progress), 1);
    dkg_state.in_progress = std::option::some(<a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a> {
        dealer_epoch,
        dealer_validator_set,
        target_epoch,
        target_validator_set,
        deadline_microseconds: <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() + 60000000,
        result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    });
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>&lt;<a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a>&gt;(
        &<b>mut</b> dkg_state.events,
        <a href="dkg.md#0x1_dkg_StartDKGEvent">StartDKGEvent</a> {
            target_epoch,
            target_validator_set,
        },
    );
}
</code></pre>



</details>

<a name="0x1_dkg_update"></a>

## Function `update`

Update the current DKG state with a potential transcript.
Return true if the current DKG becomes inactive and we should start a new epoch.
Abort if no DKG is in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <b>update</b>(dkg_result_available: bool, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <b>update</b>(dkg_result_available: bool, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&dkg_state.in_progress), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="dkg.md#0x1_dkg_ENO_RECONFIGURATION_IN_PROGRESS">ENO_RECONFIGURATION_IN_PROGRESS</a>));
    <b>let</b> session = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> dkg_state.in_progress);
    <b>let</b> dkg_completed = <b>false</b>;
    <b>if</b> (dkg_result_available) {
        session.result = dkg_result;
        dkg_completed = <b>true</b>;
    };
    <b>if</b> (<a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() &gt;= session.deadline_microseconds || dkg_completed) {
        dkg_state.last_complete = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(session);
        dkg_state.in_progress = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
        <b>true</b>
    } <b>else</b> {
        dkg_state.in_progress = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(session);
        <b>false</b>
    }
}
</code></pre>



</details>

<a name="0x1_dkg_in_progress"></a>

## Function `in_progress`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_in_progress">in_progress</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_in_progress">in_progress</a>(): bool <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<b>borrow_global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework).in_progress)
}
</code></pre>



</details>

<a name="0x1_dkg_current_deadline"></a>

## Function `current_deadline`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_current_deadline">current_deadline</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_current_deadline">current_deadline</a>(): u64 <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <b>let</b> in_progress_session = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_borrow">option::borrow</a>(&<b>borrow_global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework).in_progress);
    in_progress_session.deadline_microseconds
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
