
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


<pre><code>use 0x1::error;
use 0x1::event;
use 0x1::option;
use 0x1::randomness_config;
use 0x1::system_addresses;
use 0x1::timestamp;
use 0x1::validator_consensus_info;
</code></pre>



<a id="0x1_dkg_DKGSessionMetadata"></a>

## Struct `DKGSessionMetadata`

This can be considered as the public input of DKG.


<pre><code>struct DKGSessionMetadata has copy, drop, store
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



<pre><code>&#35;[event]
struct DKGStartEvent has drop, store
</code></pre>



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


<pre><code>struct DKGSessionState has copy, drop, store
</code></pre>



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


<pre><code>struct DKGState has key
</code></pre>



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



<pre><code>const EDKG_IN_PROGRESS: u64 &#61; 1;
</code></pre>



<a id="0x1_dkg_EDKG_NOT_IN_PROGRESS"></a>



<pre><code>const EDKG_NOT_IN_PROGRESS: u64 &#61; 2;
</code></pre>



<a id="0x1_dkg_initialize"></a>

## Function `initialize`

Called in genesis to initialize on-chain states.


<pre><code>public fun initialize(aptos_framework: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(aptos_framework: &amp;signer) &#123;
    system_addresses::assert_aptos_framework(aptos_framework);
    if (!exists&lt;DKGState&gt;(@aptos_framework)) &#123;
        move_to&lt;DKGState&gt;(
            aptos_framework,
            DKGState &#123;
                last_completed: std::option::none(),
                in_progress: std::option::none(),
            &#125;
        );
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_dkg_start"></a>

## Function `start`

Mark on-chain DKG state as in-progress. Notify validators to start DKG.
Abort if a DKG is already in progress.


<pre><code>public(friend) fun start(dealer_epoch: u64, randomness_config: randomness_config::RandomnessConfig, dealer_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;, target_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun start(
    dealer_epoch: u64,
    randomness_config: RandomnessConfig,
    dealer_validator_set: vector&lt;ValidatorConsensusInfo&gt;,
    target_validator_set: vector&lt;ValidatorConsensusInfo&gt;,
) acquires DKGState &#123;
    let dkg_state &#61; borrow_global_mut&lt;DKGState&gt;(@aptos_framework);
    let new_session_metadata &#61; DKGSessionMetadata &#123;
        dealer_epoch,
        randomness_config,
        dealer_validator_set,
        target_validator_set,
    &#125;;
    let start_time_us &#61; timestamp::now_microseconds();
    dkg_state.in_progress &#61; std::option::some(DKGSessionState &#123;
        metadata: new_session_metadata,
        start_time_us,
        transcript: vector[],
    &#125;);

    emit(DKGStartEvent &#123;
        start_time_us,
        session_metadata: new_session_metadata,
    &#125;);
&#125;
</code></pre>



</details>

<a id="0x1_dkg_finish"></a>

## Function `finish`

Put a transcript into the currently incomplete DKG session, then mark it completed.

Abort if DKG is not in progress.


<pre><code>public(friend) fun finish(transcript: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun finish(transcript: vector&lt;u8&gt;) acquires DKGState &#123;
    let dkg_state &#61; borrow_global_mut&lt;DKGState&gt;(@aptos_framework);
    assert!(option::is_some(&amp;dkg_state.in_progress), error::invalid_state(EDKG_NOT_IN_PROGRESS));
    let session &#61; option::extract(&amp;mut dkg_state.in_progress);
    session.transcript &#61; transcript;
    dkg_state.last_completed &#61; option::some(session);
    dkg_state.in_progress &#61; option::none();
&#125;
</code></pre>



</details>

<a id="0x1_dkg_try_clear_incomplete_session"></a>

## Function `try_clear_incomplete_session`

Delete the currently incomplete session, if it exists.


<pre><code>public fun try_clear_incomplete_session(fx: &amp;signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun try_clear_incomplete_session(fx: &amp;signer) acquires DKGState &#123;
    system_addresses::assert_aptos_framework(fx);
    if (exists&lt;DKGState&gt;(@aptos_framework)) &#123;
        let dkg_state &#61; borrow_global_mut&lt;DKGState&gt;(@aptos_framework);
        dkg_state.in_progress &#61; option::none();
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_dkg_incomplete_session"></a>

## Function `incomplete_session`

Return the incomplete DKG session state, if it exists.


<pre><code>public fun incomplete_session(): option::Option&lt;dkg::DKGSessionState&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun incomplete_session(): Option&lt;DKGSessionState&gt; acquires DKGState &#123;
    if (exists&lt;DKGState&gt;(@aptos_framework)) &#123;
        borrow_global&lt;DKGState&gt;(@aptos_framework).in_progress
    &#125; else &#123;
        option::none()
    &#125;
&#125;
</code></pre>



</details>

<a id="0x1_dkg_session_dealer_epoch"></a>

## Function `session_dealer_epoch`

Return the dealer epoch of a <code>DKGSessionState</code>.


<pre><code>public fun session_dealer_epoch(session: &amp;dkg::DKGSessionState): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun session_dealer_epoch(session: &amp;DKGSessionState): u64 &#123;
    session.metadata.dealer_epoch
&#125;
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;DKGState&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public fun initialize(aptos_framework: &amp;signer)
</code></pre>




<pre><code>let aptos_framework_addr &#61; signer::address_of(aptos_framework);
aborts_if aptos_framework_addr !&#61; @aptos_framework;
</code></pre>



<a id="@Specification_1_start"></a>

### Function `start`


<pre><code>public(friend) fun start(dealer_epoch: u64, randomness_config: randomness_config::RandomnessConfig, dealer_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;, target_validator_set: vector&lt;validator_consensus_info::ValidatorConsensusInfo&gt;)
</code></pre>




<pre><code>aborts_if !exists&lt;DKGState&gt;(@aptos_framework);
aborts_if !exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_finish"></a>

### Function `finish`


<pre><code>public(friend) fun finish(transcript: vector&lt;u8&gt;)
</code></pre>




<pre><code>requires exists&lt;DKGState&gt;(@aptos_framework);
requires option::is_some(global&lt;DKGState&gt;(@aptos_framework).in_progress);
aborts_if false;
</code></pre>




<a id="0x1_dkg_has_incomplete_session"></a>


<pre><code>fun has_incomplete_session(): bool &#123;
   if (exists&lt;DKGState&gt;(@aptos_framework)) &#123;
       option::spec_is_some(global&lt;DKGState&gt;(@aptos_framework).in_progress)
   &#125; else &#123;
       false
   &#125;
&#125;
</code></pre>



<a id="@Specification_1_try_clear_incomplete_session"></a>

### Function `try_clear_incomplete_session`


<pre><code>public fun try_clear_incomplete_session(fx: &amp;signer)
</code></pre>




<pre><code>let addr &#61; signer::address_of(fx);
aborts_if addr !&#61; @aptos_framework;
</code></pre>



<a id="@Specification_1_incomplete_session"></a>

### Function `incomplete_session`


<pre><code>public fun incomplete_session(): option::Option&lt;dkg::DKGSessionState&gt;
</code></pre>




<pre><code>aborts_if false;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
