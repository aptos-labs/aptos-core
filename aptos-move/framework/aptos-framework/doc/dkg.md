
<a id="0x1_dkg"></a>

# Module `0x1::dkg`

DKG configs, resources and helper functions.


-  [Struct `DKGStartEvent`](#0x1_dkg_DKGStartEvent)
-  [Struct `DKGSessionState`](#0x1_dkg_DKGSessionState)
-  [Resource `DKGState`](#0x1_dkg_DKGState)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_dkg_initialize)
-  [Function `start`](#0x1_dkg_start)
-  [Function `update`](#0x1_dkg_update)
-  [Function `in_progress`](#0x1_dkg_in_progress)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_dkg_DKGStartEvent"></a>

## Struct `DKGStartEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="dkg.md#0x1_dkg_DKGStartEvent">DKGStartEvent</a> <b>has</b> drop, store
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

<a id="0x1_dkg_DKGSessionState"></a>

## Struct `DKGSessionState`

The input and output of a DKG session.
The validator set of epoch <code>x</code> works together for an DKG output for the target validator set of epoch <code>x+1</code>.


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
<code>deadline_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_DKGState"></a>

## Resource `DKGState`

The complete and ongoing DKG sessions.


<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>last_completed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>in_progress: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_dkg_EINVALID_GUID_FOR_EVENT"></a>



<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_EINVALID_GUID_FOR_EVENT">EINVALID_GUID_FOR_EVENT</a>: u64 = 3;
</code></pre>



<a id="0x1_dkg_EDKG_IN_PROGRESS"></a>



<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_EDKG_IN_PROGRESS">EDKG_IN_PROGRESS</a>: u64 = 1;
</code></pre>



<a id="0x1_dkg_EDKG_NOT_IN_PROGRESS"></a>



<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_EDKG_NOT_IN_PROGRESS">EDKG_NOT_IN_PROGRESS</a>: u64 = 2;
</code></pre>



<a id="0x1_dkg_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>move_to</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(
        aptos_framework,
        <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
            last_completed: std::option::none(),
            in_progress: std::option::none(),
        }
    );
}
</code></pre>



</details>

<a id="0x1_dkg_start"></a>

## Function `start`

Mark on-chain DKG state as in-progress. Notify validators to start DKG.
Abort if a DKG is already in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(dealer_epoch: u64, dealer_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>, target_epoch: u64, target_validator_set: <a href="stake.md#0x1_stake_ValidatorSet">stake::ValidatorSet</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(
    dealer_epoch: u64,
    dealer_validator_set: ValidatorSet,
    target_epoch: u64,
    target_validator_set: ValidatorSet
) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <b>assert</b>!(std::option::is_none(&dkg_state.in_progress), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="dkg.md#0x1_dkg_EDKG_IN_PROGRESS">EDKG_IN_PROGRESS</a>));
    dkg_state.in_progress = std::option::some(<a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a> {
        dealer_epoch,
        dealer_validator_set,
        target_epoch,
        target_validator_set,
        deadline_secs: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>() + 9999999999, //TODO: maybe from DKG config resource
        result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    });

    <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="dkg.md#0x1_dkg_DKGStartEvent">DKGStartEvent</a> {
        target_epoch,
        target_validator_set,
    };
    emit(<a href="event.md#0x1_event">event</a>);
}
</code></pre>



</details>

<a id="0x1_dkg_update"></a>

## Function `update`

Update the current DKG state at the beginning of every block in <code>block_prologue_ext()</code>,
or when DKG result is available.

Return true if and only if this update completes/aborts the DKG and we should proceed to the next epoch.

Abort if DKG is not in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <b>update</b>(dkg_result_available: bool, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <b>update</b>(dkg_result_available: bool, dkg_result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <b>let</b> dkg_state = <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&dkg_state.in_progress), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="dkg.md#0x1_dkg_EDKG_NOT_IN_PROGRESS">EDKG_NOT_IN_PROGRESS</a>));
    <b>let</b> session = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> dkg_state.in_progress);
    <b>let</b> dkg_completed = <b>false</b>;
    <b>if</b> (dkg_result_available) {
        session.result = dkg_result;
        dkg_completed = <b>true</b>;
    };
    <b>let</b> dkg_timed_out = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() &gt;= session.deadline_secs;
    <b>if</b> (dkg_timed_out || dkg_completed) {
        dkg_state.last_completed = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(session);
        dkg_state.in_progress = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
        <b>true</b>
    } <b>else</b> {
        dkg_state.in_progress = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(session);
        <b>false</b>
    }
}
</code></pre>



</details>

<a id="0x1_dkg_in_progress"></a>

## Function `in_progress`

Return whether a DKG is in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_in_progress">in_progress</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_in_progress">in_progress</a>(): bool <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&<b>borrow_global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework).in_progress)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
