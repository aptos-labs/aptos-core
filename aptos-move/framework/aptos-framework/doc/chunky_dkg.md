
<a id="0x1_chunky_dkg"></a>

# Module `0x1::chunky_dkg`

Chunky DKG on-chain states and helper functions.


-  [Struct `ChunkyDKGSessionMetadata`](#0x1_chunky_dkg_ChunkyDKGSessionMetadata)
-  [Struct `ChunkyDKGStartEvent`](#0x1_chunky_dkg_ChunkyDKGStartEvent)
-  [Struct `ChunkyDKGSessionState`](#0x1_chunky_dkg_ChunkyDKGSessionState)
-  [Resource `ChunkyDKGState`](#0x1_chunky_dkg_ChunkyDKGState)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_chunky_dkg_initialize)
-  [Function `start`](#0x1_chunky_dkg_start)
-  [Function `finish`](#0x1_chunky_dkg_finish)
-  [Function `try_clear_incomplete_session`](#0x1_chunky_dkg_try_clear_incomplete_session)
-  [Function `incomplete_session`](#0x1_chunky_dkg_incomplete_session)
-  [Function `session_dealer_epoch`](#0x1_chunky_dkg_session_dealer_epoch)


<pre><code><b>use</b> <a href="chunky_dkg_config.md#0x1_chunky_dkg_config">0x1::chunky_dkg_config</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;
</code></pre>



<a id="0x1_chunky_dkg_ChunkyDKGSessionMetadata"></a>

## Struct `ChunkyDKGSessionMetadata`

This can be considered as the public input of Chunky DKG.


<pre><code><b>struct</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionMetadata">ChunkyDKGSessionMetadata</a> <b>has</b> <b>copy</b>, drop, store
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
<code><a href="chunky_dkg_config.md#0x1_chunky_dkg_config">chunky_dkg_config</a>: <a href="chunky_dkg_config.md#0x1_chunky_dkg_config_ChunkyDKGConfig">chunky_dkg_config::ChunkyDKGConfig</a></code>
</dt>
<dd>

</dd>
<dt>
<code>dealer_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>target_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_chunky_dkg_ChunkyDKGStartEvent"></a>

## Struct `ChunkyDKGStartEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGStartEvent">ChunkyDKGStartEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>session_metadata: <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionMetadata">chunky_dkg::ChunkyDKGSessionMetadata</a></code>
</dt>
<dd>

</dd>
<dt>
<code>start_time_us: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_chunky_dkg_ChunkyDKGSessionState"></a>

## Struct `ChunkyDKGSessionState`

The input and output of a Chunky DKG session.
The validator set of epoch <code>x</code> works together for a Chunky DKG output for the target validator set of epoch <code>x+1</code>.


<pre><code><b>struct</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">ChunkyDKGSessionState</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionMetadata">chunky_dkg::ChunkyDKGSessionMetadata</a></code>
</dt>
<dd>

</dd>
<dt>
<code>start_time_us: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>aggregated_subtranscript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_chunky_dkg_ChunkyDKGState"></a>

## Resource `ChunkyDKGState`

The completed and in-progress Chunky DKG sessions.


<pre><code><b>struct</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>last_completed: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">chunky_dkg::ChunkyDKGSessionState</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>in_progress: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">chunky_dkg::ChunkyDKGSessionState</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_chunky_dkg_ECHUNKY_DKG_IN_PROGRESS"></a>



<pre><code><b>const</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ECHUNKY_DKG_IN_PROGRESS">ECHUNKY_DKG_IN_PROGRESS</a>: u64 = 1;
</code></pre>



<a id="0x1_chunky_dkg_ECHUNKY_DKG_NOT_IN_PROGRESS"></a>



<pre><code><b>const</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ECHUNKY_DKG_NOT_IN_PROGRESS">ECHUNKY_DKG_NOT_IN_PROGRESS</a>: u64 = 2;
</code></pre>



<a id="0x1_chunky_dkg_initialize"></a>

## Function `initialize`

Called in genesis to initialize on-chain states.


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(@aptos_framework)) {
        <b>move_to</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(
            aptos_framework,
            <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a> {
                last_completed: std::option::none(),
                in_progress: std::option::none()
            }
        );
    }
}
</code></pre>



</details>

<a id="0x1_chunky_dkg_start"></a>

## Function `start`

Mark on-chain Chunky DKG state as in-progress. Notify validators to start Chunky DKG.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_start">start</a>(dealer_epoch: u64, <a href="chunky_dkg_config.md#0x1_chunky_dkg_config">chunky_dkg_config</a>: <a href="chunky_dkg_config.md#0x1_chunky_dkg_config_ChunkyDKGConfig">chunky_dkg_config::ChunkyDKGConfig</a>, dealer_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;, target_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_start">start</a>(
    dealer_epoch: u64,
    <a href="chunky_dkg_config.md#0x1_chunky_dkg_config">chunky_dkg_config</a>: ChunkyDKGConfig,
    dealer_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt;,
    target_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt;
) <b>acquires</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a> {
    <b>let</b> chunky_dkg_state = <b>borrow_global_mut</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(@aptos_framework);
    <b>let</b> new_session_metadata = <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionMetadata">ChunkyDKGSessionMetadata</a> {
        dealer_epoch,
        <a href="chunky_dkg_config.md#0x1_chunky_dkg_config">chunky_dkg_config</a>,
        dealer_validator_set,
        target_validator_set
    };
    <b>let</b> start_time_us = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    chunky_dkg_state.in_progress = std::option::some(
        <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">ChunkyDKGSessionState</a> {
            metadata: new_session_metadata,
            start_time_us,
            aggregated_subtranscript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
        }
    );

    emit(
        <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGStartEvent">ChunkyDKGStartEvent</a> { start_time_us, session_metadata: new_session_metadata }
    );
}
</code></pre>



</details>

<a id="0x1_chunky_dkg_finish"></a>

## Function `finish`

Put a transcript into the currently incomplete Chunky DKG session, then mark it completed.

Abort if Chunky DKG is not in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_finish">finish</a>(aggregated_subtranscript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_finish">finish</a>(aggregated_subtranscript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a> {
    <b>let</b> chunky_dkg_state = <b>borrow_global_mut</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(@aptos_framework);
    <b>assert</b>!(
        chunky_dkg_state.in_progress.is_some(),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="chunky_dkg.md#0x1_chunky_dkg_ECHUNKY_DKG_NOT_IN_PROGRESS">ECHUNKY_DKG_NOT_IN_PROGRESS</a>)
    );
    <b>let</b> session = chunky_dkg_state.in_progress.extract();
    session.aggregated_subtranscript = aggregated_subtranscript;
    chunky_dkg_state.last_completed = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(session);
    chunky_dkg_state.in_progress = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
}
</code></pre>



</details>

<a id="0x1_chunky_dkg_try_clear_incomplete_session"></a>

## Function `try_clear_incomplete_session`

Delete the currently incomplete session, if it exists.


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_try_clear_incomplete_session">try_clear_incomplete_session</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_try_clear_incomplete_session">try_clear_incomplete_session</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <b>if</b> (<b>exists</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(@aptos_framework)) {
        <b>let</b> chunky_dkg_state = <b>borrow_global_mut</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(@aptos_framework);
        chunky_dkg_state.in_progress = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    }
}
</code></pre>



</details>

<a id="0x1_chunky_dkg_incomplete_session"></a>

## Function `incomplete_session`

Return the incomplete Chunky DKG session state, if it exists.


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_incomplete_session">incomplete_session</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">chunky_dkg::ChunkyDKGSessionState</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_incomplete_session">incomplete_session</a>(): Option&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">ChunkyDKGSessionState</a>&gt; <b>acquires</b> <a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(@aptos_framework)) {
        <b>borrow_global</b>&lt;<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGState">ChunkyDKGState</a>&gt;(@aptos_framework).in_progress
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_chunky_dkg_session_dealer_epoch"></a>

## Function `session_dealer_epoch`

Return the dealer epoch of a <code><a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">ChunkyDKGSessionState</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_session_dealer_epoch">session_dealer_epoch</a>(session: &<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">chunky_dkg::ChunkyDKGSessionState</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="chunky_dkg.md#0x1_chunky_dkg_session_dealer_epoch">session_dealer_epoch</a>(session: &<a href="chunky_dkg.md#0x1_chunky_dkg_ChunkyDKGSessionState">ChunkyDKGSessionState</a>): u64 {
    session.metadata.dealer_epoch
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
