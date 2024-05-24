
<a id="0x1_reconfiguration_state"></a>

# Module `0x1::reconfiguration_state`

Reconfiguration meta&#45;state resources and util functions.

WARNING: <code><a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">reconfiguration_state::initialize</a>()</code> is required before <code>RECONFIGURE_WITH_DKG</code> can be enabled.


-  [Resource `State`](#0x1_reconfiguration_state_State)
-  [Struct `StateInactive`](#0x1_reconfiguration_state_StateInactive)
-  [Struct `StateActive`](#0x1_reconfiguration_state_StateActive)
-  [Constants](#@Constants_0)
-  [Function `is_initialized`](#0x1_reconfiguration_state_is_initialized)
-  [Function `initialize`](#0x1_reconfiguration_state_initialize)
-  [Function `initialize_for_testing`](#0x1_reconfiguration_state_initialize_for_testing)
-  [Function `is_in_progress`](#0x1_reconfiguration_state_is_in_progress)
-  [Function `on_reconfig_start`](#0x1_reconfiguration_state_on_reconfig_start)
-  [Function `start_time_secs`](#0x1_reconfiguration_state_start_time_secs)
-  [Function `on_reconfig_finish`](#0x1_reconfiguration_state_on_reconfig_finish)
-  [Specification](#@Specification_1)
    -  [Resource `State`](#@Specification_1_State)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `initialize_for_testing`](#@Specification_1_initialize_for_testing)
    -  [Function `is_in_progress`](#@Specification_1_is_in_progress)
    -  [Function `on_reconfig_start`](#@Specification_1_on_reconfig_start)
    -  [Function `start_time_secs`](#@Specification_1_start_time_secs)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;<br /></code></pre>



<a id="0x1_reconfiguration_state_State"></a>

## Resource `State`

Reconfiguration drivers update this resources to notify other modules of some reconfiguration state.


<pre><code><b>struct</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> <b>has</b> key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 The state variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 &#45; <code>ReconfigStateInactive</code>
 &#45; <code>ReconfigStateActive</code>
</dd>
</dl>


</details>

<a id="0x1_reconfiguration_state_StateInactive"></a>

## Struct `StateInactive`

A state variant indicating no reconfiguration is in progress.


<pre><code><b>struct</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_reconfiguration_state_StateActive"></a>

## Struct `StateActive`

A state variant indicating a reconfiguration is in progress.


<pre><code><b>struct</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>start_time_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS"></a>



<pre><code><b>const</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS">ERECONFIG_NOT_IN_PROGRESS</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_reconfiguration_state_is_initialized"></a>

## Function `is_initialized`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_initialized">is_initialized</a>(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_initialized">is_initialized</a>(): bool &#123;<br />    <b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_state_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) &#123;<br />        <b>move_to</b>(fx, <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> &#123;<br />            variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a> &#123;&#125;)<br />        &#125;)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_state_initialize_for_testing"></a>

## Function `initialize_for_testing`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize_for_testing">initialize_for_testing</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize_for_testing">initialize_for_testing</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) &#123;<br />    <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_state_is_in_progress"></a>

## Function `is_in_progress`

Return whether the reconfiguration state is marked &quot;in progress&quot;.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">is_in_progress</a>(): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">is_in_progress</a>(): bool <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> &#123;<br />    <b>if</b> (!<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) &#123;<br />        <b>return</b> <b>false</b><br />    &#125;;<br /><br />    <b>let</b> state &#61; <b>borrow_global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br />    <b>let</b> variant_type_name &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&amp;state.variant));<br />    variant_type_name &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_state_on_reconfig_start"></a>

## Function `on_reconfig_start`

Called at the beginning of a reconfiguration (either immediate or async)
to mark the reconfiguration state &quot;in progress&quot; if it is currently &quot;stopped&quot;.

Also record the current time as the reconfiguration start time. (Some module, e.g., <code><a href="stake.md#0x1_stake">stake</a>.<b>move</b></code>, needs this info).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">on_reconfig_start</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">on_reconfig_start</a>() <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> state &#61; <b>borrow_global_mut</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br />        <b>let</b> variant_type_name &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&amp;state.variant));<br />        <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>&quot;) &#123;<br />            state.variant &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a> &#123;<br />                start_time_secs: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()<br />            &#125;);<br />        &#125;<br />    &#125;;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_state_start_time_secs"></a>

## Function `start_time_secs`

Get the unix time when the currently in&#45;progress reconfiguration started.
Abort if the reconfiguration state is not &quot;in progress&quot;.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">start_time_secs</a>(): u64<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">start_time_secs</a>(): u64 <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> &#123;<br />    <b>let</b> state &#61; <b>borrow_global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br />    <b>let</b> variant_type_name &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&amp;state.variant));<br />    <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;) &#123;<br />        <b>let</b> active &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;(state.variant);<br />        active.start_time_secs<br />    &#125; <b>else</b> &#123;<br />        <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS">ERECONFIG_NOT_IN_PROGRESS</a>))<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_reconfiguration_state_on_reconfig_finish"></a>

## Function `on_reconfig_finish`

Called at the end of every reconfiguration to mark the state as &quot;stopped&quot;.
Abort if the current state is not &quot;in progress&quot;.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_finish">on_reconfig_finish</a>()<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_finish">on_reconfig_finish</a>() <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> &#123;<br />    <b>if</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) &#123;<br />        <b>let</b> state &#61; <b>borrow_global_mut</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br />        <b>let</b> variant_type_name &#61; &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&amp;state.variant));<br />        <b>if</b> (variant_type_name &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;) &#123;<br />            state.variant &#61; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a> &#123;&#125;);<br />        &#125; <b>else</b> &#123;<br />            <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS">ERECONFIG_NOT_IN_PROGRESS</a>))<br />        &#125;<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() &#61;&#61;&gt; <b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br /></code></pre>



<a id="@Specification_1_State"></a>

### Resource `State`


<pre><code><b>struct</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> <b>has</b> key<br /></code></pre>



<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 The state variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 &#45; <code>ReconfigStateInactive</code>
 &#45; <code>ReconfigStateActive</code>
</dd>
</dl>



<pre><code><b>invariant</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(variant).bytes &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot; &#124;&#124;<br />    <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(variant).bytes &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>&quot;;<br /><b>invariant</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(variant).bytes &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;<br />    &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;(variant.data);<br /><b>invariant</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(variant).bytes &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>&quot;<br />    &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a>&gt;(variant.data);<br /><b>invariant</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(variant).bytes &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot; &#61;&#61;&gt;<br />    <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;() &#61;&#61; variant.type_name;<br /><b>invariant</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(variant).bytes &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>&quot; &#61;&#61;&gt;<br />    <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a>&gt;() &#61;&#61; variant.type_name;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx) !&#61; @aptos_framework;<br /><b>let</b> <b>post</b> post_state &#61; <b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br /><b>ensures</b> <b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br /><b>ensures</b> !<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework) &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a>&gt;(post_state.variant.data);<br /></code></pre>



<a id="@Specification_1_initialize_for_testing"></a>

### Function `initialize_for_testing`


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize_for_testing">initialize_for_testing</a>(fx: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)<br /></code></pre>




<pre><code><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx) !&#61; @aptos_framework;<br /></code></pre>



<a id="@Specification_1_is_in_progress"></a>

### Function `is_in_progress`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">is_in_progress</a>(): bool<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /></code></pre>




<a id="0x1_reconfiguration_state_spec_is_in_progress"></a>


<pre><code><b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">spec_is_in_progress</a>(): bool &#123;<br />   <b>if</b> (!<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) &#123;<br />       <b>false</b><br />   &#125; <b>else</b> &#123;<br />       <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;<br />   &#125;<br />&#125;<br /></code></pre>



<a id="@Specification_1_on_reconfig_start"></a>

### Function `on_reconfig_start`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">on_reconfig_start</a>()<br /></code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;<br /><b>requires</b> <b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);<br /><b>let</b> state &#61; Any &#123;<br />    type_name: <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;(),<br />    data: <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_serialize">bcs::serialize</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a> &#123;<br />        start_time_secs: <a href="timestamp.md#0x1_timestamp_spec_now_seconds">timestamp::spec_now_seconds</a>()<br />    &#125;)<br />&#125;;<br /><b>let</b> pre_state &#61; <b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br /><b>let</b> <b>post</b> post_state &#61; <b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br /><b>ensures</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework) &amp;&amp; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(pre_state.variant).bytes<br />    &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>&quot;) &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(post_state.variant).bytes<br />    &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;;<br /><b>ensures</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework) &amp;&amp; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(pre_state.variant).bytes<br />    &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>&quot;) &#61;&#61;&gt; post_state.variant &#61;&#61; state;<br /><b>ensures</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework) &amp;&amp; <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(pre_state.variant).bytes<br />    &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>&quot;) &#61;&#61;&gt; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;(post_state.variant.data);<br /></code></pre>



<a id="@Specification_1_start_time_secs"></a>

### Function `start_time_secs`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">start_time_secs</a>(): u64<br /></code></pre>




<pre><code><b>include</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StartTimeSecsAbortsIf">StartTimeSecsAbortsIf</a>;<br /></code></pre>




<a id="0x1_reconfiguration_state_spec_start_time_secs"></a>


<pre><code><b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_start_time_secs">spec_start_time_secs</a>(): u64 &#123;<br />   <b>use</b> aptos_std::from_bcs;<br />   <b>let</b> state &#61; <b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br />   <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserialize">from_bcs::deserialize</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;(state.variant.data).start_time_secs<br />&#125;<br /></code></pre>




<a id="0x1_reconfiguration_state_StartTimeSecsRequirement"></a>


<pre><code><b>schema</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StartTimeSecsRequirement">StartTimeSecsRequirement</a> &#123;<br /><b>requires</b> <b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br /><b>requires</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes<br />    &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;;<br /><b>include</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_UnpackRequiresStateActive">UnpackRequiresStateActive</a> &#123;<br />    x:  <b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant<br />&#125;;<br />&#125;<br /></code></pre>




<a id="0x1_reconfiguration_state_UnpackRequiresStateActive"></a>


<pre><code><b>schema</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_UnpackRequiresStateActive">UnpackRequiresStateActive</a> &#123;<br />x: Any;<br /><b>requires</b> <a href="../../aptos-stdlib/doc/type_info.md#0x1_type_info_type_name">type_info::type_name</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;() &#61;&#61; x.type_name &amp;&amp; <a href="../../aptos-stdlib/doc/from_bcs.md#0x1_from_bcs_deserializable">from_bcs::deserializable</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;(x.data);<br />&#125;<br /></code></pre>




<a id="0x1_reconfiguration_state_StartTimeSecsAbortsIf"></a>


<pre><code><b>schema</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StartTimeSecsAbortsIf">StartTimeSecsAbortsIf</a> &#123;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);<br /><b>include</b>  <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes<br />    &#61;&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot; &#61;&#61;&gt;<br /><a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_UnpackAbortsIf">copyable_any::UnpackAbortsIf</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt; &#123;<br />    x:  <b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant<br />&#125;;<br /><b>aborts_if</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes<br />    !&#61; b&quot;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>&quot;;<br />&#125;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
