
<a id="0x1_dkg"></a>

# Module `0x1::dkg`

DKG on&#45;chain states and helper functions.


-  [Struct `DKGSessionMetadata`](#0x1_dkg_DKGSessionMetadata)
-  [Struct `DKGStartEvent`](#0x1_dkg_DKGStartEvent)
-  [Struct `DKGSessionState`](#0x1_dkg_DKGSessionState)
-  [Resource `DKGState`](#0x1_dkg_DKGState)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_dkg_initialize)
-  [Function `start`](#0x1_dkg_start)
-  [Function `finish`](#0x1_dkg_finish)
-  [Function `try_clear_incomplete_session`](#0x1_dkg_try_clear_incomplete_session)
-  [Function `incomplete_session`](#0x1_dkg_incomplete_session)
-  [Function `session_dealer_epoch`](#0x1_dkg_session_dealer_epoch)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `start`](#@Specification_1_start)
    -  [Function `finish`](#@Specification_1_finish)
    -  [Function `try_clear_incomplete_session`](#@Specification_1_try_clear_incomplete_session)
    -  [Function `incomplete_session`](#@Specification_1_incomplete_session)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="randomness_config.md#0x1_randomness_config">0x1::randomness_config</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /><b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;<br /></code></pre>



<a id="0x1_dkg_DKGSessionMetadata"></a>

## Struct `DKGSessionMetadata`

This can be considered as the public input of DKG.


<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_DKGSessionMetadata">DKGSessionMetadata</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dealer_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="randomness_config.md#0x1_randomness_config">randomness_config</a>: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a></code>
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

<a id="0x1_dkg_DKGStartEvent"></a>

## Struct `DKGStartEvent`



<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="dkg.md#0x1_dkg_DKGStartEvent">DKGStartEvent</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>session_metadata: <a href="dkg.md#0x1_dkg_DKGSessionMetadata">dkg::DKGSessionMetadata</a></code>
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

<a id="0x1_dkg_DKGSessionState"></a>

## Struct `DKGSessionState`

The input and output of a DKG session.
The validator set of epoch <code>x</code> works together for an DKG output for the target validator set of epoch <code>x&#43;1</code>.


<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: <a href="dkg.md#0x1_dkg_DKGSessionMetadata">dkg::DKGSessionMetadata</a></code>
</dt>
<dd>

</dd>
<dt>
<code>start_time_us: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_DKGState"></a>

## Resource `DKGState`

The completed and in&#45;progress DKG sessions.


<pre><code><b>struct</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> <b>has</b> key<br /></code></pre>



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


<a id="0x1_dkg_EDKG_IN_PROGRESS"></a>



<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_EDKG_IN_PROGRESS">EDKG_IN_PROGRESS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_dkg_EDKG_NOT_IN_PROGRESS"></a>



<pre><code><b>const</b> <a href="dkg.md#0x1_dkg_EDKG_NOT_IN_PROGRESS">EDKG_NOT_IN_PROGRESS</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_dkg_initialize"></a>

## Function `initialize`

Called in genesis to initialize on&#45;chain states.


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework)) &#123;<br />        <b>move_to</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(<br />            aptos_framework,<br />            <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> &#123;<br />                last_completed: std::option::none(),<br />                in_progress: std::option::none(),<br />            &#125;<br />        );<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dkg_start"></a>

## Function `start`

Mark on&#45;chain DKG state as in&#45;progress. Notify validators to start DKG.
Abort if a DKG is already in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(dealer_epoch: u64, <a href="randomness_config.md#0x1_randomness_config">randomness_config</a>: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>, dealer_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;, target_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(<br />    dealer_epoch: u64,<br />    <a href="randomness_config.md#0x1_randomness_config">randomness_config</a>: RandomnessConfig,<br />    dealer_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt;,<br />    target_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;ValidatorConsensusInfo&gt;,<br />) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> &#123;<br />    <b>let</b> dkg_state &#61; <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);<br />    <b>let</b> new_session_metadata &#61; <a href="dkg.md#0x1_dkg_DKGSessionMetadata">DKGSessionMetadata</a> &#123;<br />        dealer_epoch,<br />        <a href="randomness_config.md#0x1_randomness_config">randomness_config</a>,<br />        dealer_validator_set,<br />        target_validator_set,<br />    &#125;;<br />    <b>let</b> start_time_us &#61; <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();<br />    dkg_state.in_progress &#61; std::option::some(<a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a> &#123;<br />        metadata: new_session_metadata,<br />        start_time_us,<br />        transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],<br />    &#125;);<br /><br />    emit(<a href="dkg.md#0x1_dkg_DKGStartEvent">DKGStartEvent</a> &#123;<br />        start_time_us,<br />        session_metadata: new_session_metadata,<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dkg_finish"></a>

## Function `finish`

Put a transcript into the currently incomplete DKG session, then mark it completed.

Abort if DKG is not in progress.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_finish">finish</a>(transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_finish">finish</a>(transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> &#123;<br />    <b>let</b> dkg_state &#61; <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);<br />    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&amp;dkg_state.in_progress), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="dkg.md#0x1_dkg_EDKG_NOT_IN_PROGRESS">EDKG_NOT_IN_PROGRESS</a>));<br />    <b>let</b> session &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&amp;<b>mut</b> dkg_state.in_progress);<br />    session.transcript &#61; transcript;<br />    dkg_state.last_completed &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(session);<br />    dkg_state.in_progress &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dkg_try_clear_incomplete_session"></a>

## Function `try_clear_incomplete_session`

Delete the currently incomplete session, if it exists.


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_try_clear_incomplete_session">try_clear_incomplete_session</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_try_clear_incomplete_session">try_clear_incomplete_session</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <b>if</b> (<b>exists</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> dkg_state &#61; <b>borrow_global_mut</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);<br />        dkg_state.in_progress &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dkg_incomplete_session"></a>

## Function `incomplete_session`

Return the incomplete DKG session state, if it exists.


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_incomplete_session">incomplete_session</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_incomplete_session">incomplete_session</a>(): Option&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a>&gt; <b>acquires</b> <a href="dkg.md#0x1_dkg_DKGState">DKGState</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework)) &#123;<br />        <b>borrow_global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework).in_progress<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_dkg_session_dealer_epoch"></a>

## Function `session_dealer_epoch`

Return the dealer epoch of a <code><a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_session_dealer_epoch">session_dealer_epoch</a>(session: &amp;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_session_dealer_epoch">session_dealer_epoch</a>(session: &amp;<a href="dkg.md#0x1_dkg_DKGSessionState">DKGSessionState</a>): u64 &#123;<br />    session.metadata.dealer_epoch<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>let</b> aptos_framework_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>aborts_if</b> aptos_framework_addr !&#61; @aptos_framework;<br /></code></pre>



<a id="@Specification_1_start"></a>

### Function `start`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_start">start</a>(dealer_epoch: u64, <a href="randomness_config.md#0x1_randomness_config">randomness_config</a>: <a href="randomness_config.md#0x1_randomness_config_RandomnessConfig">randomness_config::RandomnessConfig</a>, dealer_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;, target_validator_set: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;)<br /></code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_finish"></a>

### Function `finish`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dkg.md#0x1_dkg_finish">finish</a>(transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)<br /></code></pre>




<pre><code><b>requires</b> <b>exists</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework);<br /><b>requires</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(<b>global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework).in_progress);<br /><b>aborts_if</b> <b>false</b>;<br /></code></pre>




<a id="0x1_dkg_has_incomplete_session"></a>


<pre><code><b>fun</b> <a href="dkg.md#0x1_dkg_has_incomplete_session">has_incomplete_session</a>(): bool &#123;<br />   <b>if</b> (<b>exists</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework)) &#123;<br />       <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_spec_is_some">option::spec_is_some</a>(<b>global</b>&lt;<a href="dkg.md#0x1_dkg_DKGState">DKGState</a>&gt;(@aptos_framework).in_progress)<br />   &#125; <b>else</b> &#123;<br />       <b>false</b><br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_try_clear_incomplete_session"></a>

### Function `try_clear_incomplete_session`


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_try_clear_incomplete_session">try_clear_incomplete_session</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx);<br /><b>aborts_if</b> addr !&#61; @aptos_framework;<br /></code></pre>



<a id="@Specification_1_incomplete_session"></a>

### Function `incomplete_session`


<pre><code><b>public</b> <b>fun</b> <a href="dkg.md#0x1_dkg_incomplete_session">incomplete_session</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="dkg.md#0x1_dkg_DKGSessionState">dkg::DKGSessionState</a>&gt;<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
