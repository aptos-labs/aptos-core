
<a id="0x1_reconfiguration_state"></a>

# Module `0x1::reconfiguration_state`

Reconfiguration meta&#45;state resources and util functions.<br/><br/> WARNING: <code>reconfiguration_state::initialize()</code> is required before <code>RECONFIGURE_WITH_DKG</code> can be enabled.


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


<pre><code>use 0x1::copyable_any;<br/>use 0x1::error;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/>use 0x1::timestamp;<br/></code></pre>



<a id="0x1_reconfiguration_state_State"></a>

## Resource `State`

Reconfiguration drivers update this resources to notify other modules of some reconfiguration state.


<pre><code>struct State has key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: copyable_any::Any</code>
</dt>
<dd>
 The state variant packed as an <code>Any</code>.<br/> Currently the variant type is one of the following.<br/> &#45; <code>ReconfigStateInactive</code><br/> &#45; <code>ReconfigStateActive</code>
</dd>
</dl>


</details>

<a id="0x1_reconfiguration_state_StateInactive"></a>

## Struct `StateInactive`

A state variant indicating no reconfiguration is in progress.


<pre><code>struct StateInactive has copy, drop, store<br/></code></pre>



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


<pre><code>struct StateActive has copy, drop, store<br/></code></pre>



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



<pre><code>const ERECONFIG_NOT_IN_PROGRESS: u64 &#61; 1;<br/></code></pre>



<a id="0x1_reconfiguration_state_is_initialized"></a>

## Function `is_initialized`



<pre><code>public fun is_initialized(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun is_initialized(): bool &#123;<br/>    exists&lt;State&gt;(@aptos_framework)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_state_initialize"></a>

## Function `initialize`



<pre><code>public fun initialize(fx: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize(fx: &amp;signer) &#123;<br/>    system_addresses::assert_aptos_framework(fx);<br/>    if (!exists&lt;State&gt;(@aptos_framework)) &#123;<br/>        move_to(fx, State &#123;<br/>            variant: copyable_any::pack(StateInactive &#123;&#125;)<br/>        &#125;)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_state_initialize_for_testing"></a>

## Function `initialize_for_testing`



<pre><code>public fun initialize_for_testing(fx: &amp;signer)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun initialize_for_testing(fx: &amp;signer) &#123;<br/>    initialize(fx)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_state_is_in_progress"></a>

## Function `is_in_progress`

Return whether the reconfiguration state is marked &quot;in progress&quot;.


<pre><code>public(friend) fun is_in_progress(): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun is_in_progress(): bool acquires State &#123;<br/>    if (!exists&lt;State&gt;(@aptos_framework)) &#123;<br/>        return false<br/>    &#125;;<br/><br/>    let state &#61; borrow_global&lt;State&gt;(@aptos_framework);<br/>    let variant_type_name &#61; &#42;string::bytes(copyable_any::type_name(&amp;state.variant));<br/>    variant_type_name &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_state_on_reconfig_start"></a>

## Function `on_reconfig_start`

Called at the beginning of a reconfiguration (either immediate or async)<br/> to mark the reconfiguration state &quot;in progress&quot; if it is currently &quot;stopped&quot;.<br/><br/> Also record the current time as the reconfiguration start time. (Some module, e.g., <code>stake.move</code>, needs this info).


<pre><code>public(friend) fun on_reconfig_start()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_reconfig_start() acquires State &#123;<br/>    if (exists&lt;State&gt;(@aptos_framework)) &#123;<br/>        let state &#61; borrow_global_mut&lt;State&gt;(@aptos_framework);<br/>        let variant_type_name &#61; &#42;string::bytes(copyable_any::type_name(&amp;state.variant));<br/>        if (variant_type_name &#61;&#61; b&quot;0x1::reconfiguration_state::StateInactive&quot;) &#123;<br/>            state.variant &#61; copyable_any::pack(StateActive &#123;<br/>                start_time_secs: timestamp::now_seconds()<br/>            &#125;);<br/>        &#125;<br/>    &#125;;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_state_start_time_secs"></a>

## Function `start_time_secs`

Get the unix time when the currently in&#45;progress reconfiguration started.<br/> Abort if the reconfiguration state is not &quot;in progress&quot;.


<pre><code>public(friend) fun start_time_secs(): u64<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun start_time_secs(): u64 acquires State &#123;<br/>    let state &#61; borrow_global&lt;State&gt;(@aptos_framework);<br/>    let variant_type_name &#61; &#42;string::bytes(copyable_any::type_name(&amp;state.variant));<br/>    if (variant_type_name &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;) &#123;<br/>        let active &#61; copyable_any::unpack&lt;StateActive&gt;(state.variant);<br/>        active.start_time_secs<br/>    &#125; else &#123;<br/>        abort(error::invalid_state(ERECONFIG_NOT_IN_PROGRESS))<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_reconfiguration_state_on_reconfig_finish"></a>

## Function `on_reconfig_finish`

Called at the end of every reconfiguration to mark the state as &quot;stopped&quot;.<br/> Abort if the current state is not &quot;in progress&quot;.


<pre><code>public(friend) fun on_reconfig_finish()<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public(friend) fun on_reconfig_finish() acquires State &#123;<br/>    if (exists&lt;State&gt;(@aptos_framework)) &#123;<br/>        let state &#61; borrow_global_mut&lt;State&gt;(@aptos_framework);<br/>        let variant_type_name &#61; &#42;string::bytes(copyable_any::type_name(&amp;state.variant));<br/>        if (variant_type_name &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;) &#123;<br/>            state.variant &#61; copyable_any::pack(StateInactive &#123;&#125;);<br/>        &#125; else &#123;<br/>            abort(error::invalid_state(ERECONFIG_NOT_IN_PROGRESS))<br/>        &#125;<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code>invariant [suspendable] chain_status::is_operating() &#61;&#61;&gt; exists&lt;State&gt;(@aptos_framework);<br/></code></pre>



<a id="@Specification_1_State"></a>

### Resource `State`


<pre><code>struct State has key<br/></code></pre>



<dl>
<dt>
<code>variant: copyable_any::Any</code>
</dt>
<dd>
 The state variant packed as an <code>Any</code>.<br/> Currently the variant type is one of the following.<br/> &#45; <code>ReconfigStateInactive</code><br/> &#45; <code>ReconfigStateActive</code>
</dd>
</dl>



<pre><code>invariant copyable_any::type_name(variant).bytes &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot; &#124;&#124;<br/>    copyable_any::type_name(variant).bytes &#61;&#61; b&quot;0x1::reconfiguration_state::StateInactive&quot;;<br/>invariant copyable_any::type_name(variant).bytes &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;<br/>    &#61;&#61;&gt; from_bcs::deserializable&lt;StateActive&gt;(variant.data);<br/>invariant copyable_any::type_name(variant).bytes &#61;&#61; b&quot;0x1::reconfiguration_state::StateInactive&quot;<br/>    &#61;&#61;&gt; from_bcs::deserializable&lt;StateInactive&gt;(variant.data);<br/>invariant copyable_any::type_name(variant).bytes &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot; &#61;&#61;&gt;<br/>    type_info::type_name&lt;StateActive&gt;() &#61;&#61; variant.type_name;<br/>invariant copyable_any::type_name(variant).bytes &#61;&#61; b&quot;0x1::reconfiguration_state::StateInactive&quot; &#61;&#61;&gt;<br/>    type_info::type_name&lt;StateInactive&gt;() &#61;&#61; variant.type_name;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>public fun initialize(fx: &amp;signer)<br/></code></pre>




<pre><code>aborts_if signer::address_of(fx) !&#61; @aptos_framework;<br/>let post post_state &#61; global&lt;State&gt;(@aptos_framework);<br/>ensures exists&lt;State&gt;(@aptos_framework);<br/>ensures !exists&lt;State&gt;(@aptos_framework) &#61;&#61;&gt; from_bcs::deserializable&lt;StateInactive&gt;(post_state.variant.data);<br/></code></pre>



<a id="@Specification_1_initialize_for_testing"></a>

### Function `initialize_for_testing`


<pre><code>public fun initialize_for_testing(fx: &amp;signer)<br/></code></pre>




<pre><code>aborts_if signer::address_of(fx) !&#61; @aptos_framework;<br/></code></pre>



<a id="@Specification_1_is_in_progress"></a>

### Function `is_in_progress`


<pre><code>public(friend) fun is_in_progress(): bool<br/></code></pre>




<pre><code>aborts_if false;<br/></code></pre>




<a id="0x1_reconfiguration_state_spec_is_in_progress"></a>


<pre><code>fun spec_is_in_progress(): bool &#123;<br/>   if (!exists&lt;State&gt;(@aptos_framework)) &#123;<br/>       false<br/>   &#125; else &#123;<br/>       copyable_any::type_name(global&lt;State&gt;(@aptos_framework).variant).bytes &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;<br/>   &#125;<br/>&#125;<br/></code></pre>



<a id="@Specification_1_on_reconfig_start"></a>

### Function `on_reconfig_start`


<pre><code>public(friend) fun on_reconfig_start()<br/></code></pre>




<pre><code>aborts_if false;<br/>requires exists&lt;timestamp::CurrentTimeMicroseconds&gt;(@aptos_framework);<br/>let state &#61; Any &#123;<br/>    type_name: type_info::type_name&lt;StateActive&gt;(),<br/>    data: bcs::serialize(StateActive &#123;<br/>        start_time_secs: timestamp::spec_now_seconds()<br/>    &#125;)<br/>&#125;;<br/>let pre_state &#61; global&lt;State&gt;(@aptos_framework);<br/>let post post_state &#61; global&lt;State&gt;(@aptos_framework);<br/>ensures (exists&lt;State&gt;(@aptos_framework) &amp;&amp; copyable_any::type_name(pre_state.variant).bytes<br/>    &#61;&#61; b&quot;0x1::reconfiguration_state::StateInactive&quot;) &#61;&#61;&gt; copyable_any::type_name(post_state.variant).bytes<br/>    &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;;<br/>ensures (exists&lt;State&gt;(@aptos_framework) &amp;&amp; copyable_any::type_name(pre_state.variant).bytes<br/>    &#61;&#61; b&quot;0x1::reconfiguration_state::StateInactive&quot;) &#61;&#61;&gt; post_state.variant &#61;&#61; state;<br/>ensures (exists&lt;State&gt;(@aptos_framework) &amp;&amp; copyable_any::type_name(pre_state.variant).bytes<br/>    &#61;&#61; b&quot;0x1::reconfiguration_state::StateInactive&quot;) &#61;&#61;&gt; from_bcs::deserializable&lt;StateActive&gt;(post_state.variant.data);<br/></code></pre>



<a id="@Specification_1_start_time_secs"></a>

### Function `start_time_secs`


<pre><code>public(friend) fun start_time_secs(): u64<br/></code></pre>




<pre><code>include StartTimeSecsAbortsIf;<br/></code></pre>




<a id="0x1_reconfiguration_state_spec_start_time_secs"></a>


<pre><code>fun spec_start_time_secs(): u64 &#123;<br/>   use aptos_std::from_bcs;<br/>   let state &#61; global&lt;State&gt;(@aptos_framework);<br/>   from_bcs::deserialize&lt;StateActive&gt;(state.variant.data).start_time_secs<br/>&#125;<br/></code></pre>




<a id="0x1_reconfiguration_state_StartTimeSecsRequirement"></a>


<pre><code>schema StartTimeSecsRequirement &#123;<br/>requires exists&lt;State&gt;(@aptos_framework);<br/>requires copyable_any::type_name(global&lt;State&gt;(@aptos_framework).variant).bytes<br/>    &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;;<br/>include UnpackRequiresStateActive &#123;<br/>    x:  global&lt;State&gt;(@aptos_framework).variant<br/>&#125;;<br/>&#125;<br/></code></pre>




<a id="0x1_reconfiguration_state_UnpackRequiresStateActive"></a>


<pre><code>schema UnpackRequiresStateActive &#123;<br/>x: Any;<br/>requires type_info::type_name&lt;StateActive&gt;() &#61;&#61; x.type_name &amp;&amp; from_bcs::deserializable&lt;StateActive&gt;(x.data);<br/>&#125;<br/></code></pre>




<a id="0x1_reconfiguration_state_StartTimeSecsAbortsIf"></a>


<pre><code>schema StartTimeSecsAbortsIf &#123;<br/>aborts_if !exists&lt;State&gt;(@aptos_framework);<br/>include  copyable_any::type_name(global&lt;State&gt;(@aptos_framework).variant).bytes<br/>    &#61;&#61; b&quot;0x1::reconfiguration_state::StateActive&quot; &#61;&#61;&gt;<br/>copyable_any::UnpackAbortsIf&lt;StateActive&gt; &#123;<br/>    x:  global&lt;State&gt;(@aptos_framework).variant<br/>&#125;;<br/>aborts_if copyable_any::type_name(global&lt;State&gt;(@aptos_framework).variant).bytes<br/>    !&#61; b&quot;0x1::reconfiguration_state::StateActive&quot;;<br/>&#125;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
