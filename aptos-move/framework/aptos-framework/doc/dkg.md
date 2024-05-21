
<a id="0x1_dkg"></a>

# Module `0x1::dkg`

DKG on-chain states and helper functions.


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


<pre><code>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::option;<br/>use 0x1::randomness_config;<br/>use 0x1::system_addresses;<br/>use 0x1::timestamp;<br/>use 0x1::validator_consensus_info;<br/></code></pre>



<a id="0x1_dkg_DKGSessionMetadata"></a>

## Struct `DKGSessionMetadata`

This can be considered as the public input of DKG.


<pre><code>struct DKGSessionMetadata has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dealer_epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>randomness_config: randomness_config::RandomnessConfig</code>
</dt>
<dd>

</dd>
<dt>
<code>dealer_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>target_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_DKGStartEvent"></a>

## Struct `DKGStartEvent`



<pre><code>&#35;[event]<br/>struct DKGStartEvent has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>session_metadata: dkg::DKGSessionMetadata</code>
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


<pre><code>struct DKGSessionState has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata: dkg::DKGSessionMetadata</code>
</dt>
<dd>

</dd>
<dt>
<code>start_time_us: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>transcript: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_dkg_DKGState"></a>

## Resource `DKGState`

The completed and in-progress DKG sessions.


<pre><code>struct DKGState has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>last_completed: option::Option&lt;dkg::DKGSessionState&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>in_progress: option::Option&lt;dkg::DKGSessionState&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_dkg_EDKG_IN_PROGRESS"></a>



<pre><code>const EDKG_IN_PROGRESS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_dkg_EDKG_NOT_IN_PROGRESS"></a>



<pre><code>const EDKG_NOT_IN_PROGRESS: u64 &#61; 2;<br/></code></pre>



<a id="0x1_dkg_initialize"></a>

## Function `initialize`

Called in genesis to initialize on-chain states.


<pre><code>public fun initialize(aptos_framework: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(aptos_framework: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    if (!exists&lt;DKGState&gt;(@aptos_framework)) &#123;<br/>        move_to&lt;DKGState&gt;(<br/>            aptos_framework,<br/>            DKGState &#123;<br/>                last_completed: std::option::none(),<br/>                in_progress: std::option::none(),<br/>            &#125;<br/>        );<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dkg_start"></a>

## Function `start`

Mark on-chain DKG state as in-progress. Notify validators to start DKG.
Abort if a DKG is already in progress.


<pre><code>public(friend) fun start(dealer_epoch: u64, randomness_config: randomness_config::RandomnessConfig, dealer_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;, target_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun start(<br/>    dealer_epoch: u64,<br/>    randomness_config: RandomnessConfig,<br/>    dealer_validator_set: vector&lt;ValidatorConsensusInfo&gt;,<br/>    target_validator_set: vector&lt;ValidatorConsensusInfo&gt;,<br/>) acquires DKGState &#123;<br/>    let dkg_state &#61; borrow_global_mut&lt;DKGState&gt;(@aptos_framework);<br/>    let new_session_metadata &#61; DKGSessionMetadata &#123;<br/>        dealer_epoch,<br/>        randomness_config,<br/>        dealer_validator_set,<br/>        target_validator_set,<br/>    &#125;;<br/>    let start_time_us &#61; timestamp::now_microseconds();<br/>    dkg_state.in_progress &#61; std::option::some(DKGSessionState &#123;<br/>        metadata: new_session_metadata,<br/>        start_time_us,<br/>        transcript: vector[],<br/>    &#125;);<br/><br/>    emit(DKGStartEvent &#123;<br/>        start_time_us,<br/>        session_metadata: new_session_metadata,<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dkg_finish"></a>

## Function `finish`

Put a transcript into the currently incomplete DKG session, then mark it completed.

Abort if DKG is not in progress.


<pre><code>public(friend) fun finish(transcript: vector&lt;u8&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun finish(transcript: vector&lt;u8&gt;) acquires DKGState &#123;<br/>    let dkg_state &#61; borrow_global_mut&lt;DKGState&gt;(@aptos_framework);<br/>    assert!(option::is_some(&amp;dkg_state.in_progress), error::invalid_state(EDKG_NOT_IN_PROGRESS));<br/>    let session &#61; option::extract(&amp;mut dkg_state.in_progress);<br/>    session.transcript &#61; transcript;<br/>    dkg_state.last_completed &#61; option::some(session);<br/>    dkg_state.in_progress &#61; option::none();<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dkg_try_clear_incomplete_session"></a>

## Function `try_clear_incomplete_session`

Delete the currently incomplete session, if it exists.


<pre><code>public fun try_clear_incomplete_session(fx: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun try_clear_incomplete_session(fx: &amp;signer) acquires DKGState &#123;<br/>    system_addresses::assert_aptos_framework(fx);<br/>    if (exists&lt;DKGState&gt;(@aptos_framework)) &#123;<br/>        let dkg_state &#61; borrow_global_mut&lt;DKGState&gt;(@aptos_framework);<br/>        dkg_state.in_progress &#61; option::none();<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dkg_incomplete_session"></a>

## Function `incomplete_session`

Return the incomplete DKG session state, if it exists.


<pre><code>public fun incomplete_session(): option::Option&lt;dkg::DKGSessionState&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun incomplete_session(): Option&lt;DKGSessionState&gt; acquires DKGState &#123;<br/>    if (exists&lt;DKGState&gt;(@aptos_framework)) &#123;<br/>        borrow_global&lt;DKGState&gt;(@aptos_framework).in_progress<br/>    &#125; else &#123;<br/>        option::none()<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_dkg_session_dealer_epoch"></a>

## Function `session_dealer_epoch`

Return the dealer epoch of a <code>DKGSessionState</code>.


<pre><code>public fun session_dealer_epoch(session: &amp;dkg::DKGSessionState): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun session_dealer_epoch(session: &amp;DKGSessionState): u64 &#123;<br/>    session.metadata.dealer_epoch<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;DKGState&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public fun initialize(aptos_framework: &amp;signer)<br/></code></pre>




<pre><code>let aptos_framework_addr &#61; signer::address_of(aptos_framework);<br/>aborts_if aptos_framework_addr !&#61; @aptos_framework;<br/></code></pre>



<a id="@Specification_1_start"></a>

### Function `start`


<pre><code>public(friend) fun start(dealer_epoch: u64, randomness_config: randomness_config::RandomnessConfig, dealer_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;, target_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;)<br/></code></pre>




<pre><code>aborts_if !exists&lt;DKGState&gt;(@aptos_framework);<br/>aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_finish"></a>

### Function `finish`


<pre><code>public(friend) fun finish(transcript: vector&lt;u8&gt;)<br/></code></pre>




<pre><code>requires exists&lt;DKGState&gt;(@aptos_framework);<br/>requires option::is_some(global&lt;DKGState&gt;(@aptos_framework).in_progress);<br/>aborts_if false;<br/></code></pre>




<a id="0x1_dkg_has_incomplete_session"></a>


<pre><code>fun has_incomplete_session(): bool &#123;<br/>   if (exists&lt;DKGState&gt;(@aptos_framework)) &#123;<br/>       option::spec_is_some(global&lt;DKGState&gt;(@aptos_framework).in_progress)<br/>   &#125; else &#123;<br/>       false<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_try_clear_incomplete_session"></a>

### Function `try_clear_incomplete_session`


<pre><code>public fun try_clear_incomplete_session(fx: &amp;signer)<br/></code></pre>




<pre><code>let addr &#61; signer::address_of(fx);<br/>aborts_if addr !&#61; @aptos_framework;<br/></code></pre>



<a id="@Specification_1_incomplete_session"></a>

### Function `incomplete_session`


<pre><code>public fun incomplete_session(): option::Option&lt;dkg::DKGSessionState&gt;<br/></code></pre>




<pre><code>aborts_if false;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
