
<a id="0x1_reconfiguration_state"></a>

# Module `0x1::reconfiguration_state`

Reconfiguration meta-state resources and util functions.


-  [Resource `State`](#0x1_reconfiguration_state_State)
-  [Struct `StateInactive`](#0x1_reconfiguration_state_StateInactive)
-  [Struct `StateActive`](#0x1_reconfiguration_state_StateActive)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_reconfiguration_state_initialize)
-  [Function `initialize_for_testing`](#0x1_reconfiguration_state_initialize_for_testing)
-  [Function `is_in_progress`](#0x1_reconfiguration_state_is_in_progress)
-  [Function `try_mark_as_in_progress`](#0x1_reconfiguration_state_try_mark_as_in_progress)
-  [Function `start_time_secs`](#0x1_reconfiguration_state_start_time_secs)
-  [Function `mark_as_completed`](#0x1_reconfiguration_state_mark_as_completed)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `initialize_for_testing`](#@Specification_1_initialize_for_testing)
    -  [Function `is_in_progress`](#@Specification_1_is_in_progress)
    -  [Function `try_mark_as_in_progress`](#@Specification_1_try_mark_as_in_progress)
    -  [Function `start_time_secs`](#@Specification_1_start_time_secs)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x1_reconfiguration_state_State"></a>

## Resource `State`

Reconfiguration drivers update this resources to notify other modules of some reconfiguration state.


<pre><code><b>struct</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>
 The state variant packed as an <code>Any</code>.
 Currently the variant type is one of the following.
 - <code>ReconfigStateInactive</code>
 - <code>ReconfigStateActive</code>
</dd>
</dl>


</details>

<a id="0x1_reconfiguration_state_StateInactive"></a>

## Struct `StateInactive`

A state variant indicating no reconfiguration is in progress.


<pre><code><b>struct</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



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


<pre><code><b>struct</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



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



<pre><code><b>const</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS">ERECONFIG_NOT_IN_PROGRESS</a>: u64 = 1;
</code></pre>



<a id="0x1_reconfiguration_state_initialize"></a>

## Function `initialize`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(fx);
    <b>if</b> (!<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(fx, <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> {
            variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a> {})
        })
    }
}
</code></pre>



</details>

<a id="0x1_reconfiguration_state_initialize_for_testing"></a>

## Function `initialize_for_testing`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize_for_testing">initialize_for_testing</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize_for_testing">initialize_for_testing</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx)
}
</code></pre>



</details>

<a id="0x1_reconfiguration_state_is_in_progress"></a>

## Function `is_in_progress`

Return whether the reconfiguration state is marked "in progress".


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">is_in_progress</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">is_in_progress</a>(): bool <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) {
        <b>return</b> <b>false</b>
    };

    <b>let</b> state = <b>borrow_global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
    <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&state.variant));
    variant_type_name == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>"
}
</code></pre>



</details>

<a id="0x1_reconfiguration_state_try_mark_as_in_progress"></a>

## Function `try_mark_as_in_progress`

Mark the reconfiguration state "in progress" if it is currently "stopped".
The current time is also recorded as the reconfiguration start time. (Some module, e.g., <code><a href="stake.md#0x1_stake">stake</a>.<b>move</b></code>, needs this info).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_try_mark_as_in_progress">try_mark_as_in_progress</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_try_mark_as_in_progress">try_mark_as_in_progress</a>() <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> {
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
    <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&state.variant));
    <b>if</b> (variant_type_name == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>") {
        state.variant = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a> {
            start_time_secs: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()
        });
    };
}
</code></pre>



</details>

<a id="0x1_reconfiguration_state_start_time_secs"></a>

## Function `start_time_secs`

Get the unix time when the currently in-progress reconfiguration started.
Abort if the reconfiguration state is not "in progress".


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">start_time_secs</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">start_time_secs</a>(): u64 <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> {
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
    <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&state.variant));
    <b>if</b> (variant_type_name == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>") {
        <b>let</b> active = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_unpack">copyable_any::unpack</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt;(state.variant);
        active.start_time_secs
    } <b>else</b> {
        <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS">ERECONFIG_NOT_IN_PROGRESS</a>))
    }
}
</code></pre>



</details>

<a id="0x1_reconfiguration_state_mark_as_completed"></a>

## Function `mark_as_completed`

Called at the end of every reconfiguration to mark the state as "stopped".
Abort if the current state is not "in progress".


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_mark_as_completed">mark_as_completed</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_mark_as_completed">mark_as_completed</a>() <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> {
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
    <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&state.variant));
    <b>if</b> (variant_type_name == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>") {
        state.variant = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a> {});
    } <b>else</b> {
        <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS">ERECONFIG_NOT_IN_PROGRESS</a>))
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>invariant</b> [suspendable] <a href="chain_status.md#0x1_chain_status_is_operating">chain_status::is_operating</a>() ==&gt; <b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
</code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx) != @aptos_framework;
</code></pre>



<a id="@Specification_1_initialize_for_testing"></a>

### Function `initialize_for_testing`


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize_for_testing">initialize_for_testing</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>aborts_if</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(fx) != @aptos_framework;
</code></pre>



<a id="@Specification_1_is_in_progress"></a>

### Function `is_in_progress`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_in_progress">is_in_progress</a>(): bool
</code></pre>




<pre><code><b>aborts_if</b> <b>false</b>;
</code></pre>




<a id="0x1_reconfiguration_state_spec_is_in_progress"></a>


<pre><code><b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_spec_is_in_progress">spec_is_in_progress</a>(): bool {
   <b>if</b> (!<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) {
       <b>false</b>
   } <b>else</b> {
       <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>"
   }
}
</code></pre>



<a id="@Specification_1_try_mark_as_in_progress"></a>

### Function `try_mark_as_in_progress`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_try_mark_as_in_progress">try_mark_as_in_progress</a>()
</code></pre>




<pre><code><b>include</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_TryMarkAsInProgressAbortsIf">TryMarkAsInProgressAbortsIf</a>;
</code></pre>




<a id="0x1_reconfiguration_state_TryMarkAsInProgressAbortsIf"></a>


<pre><code><b>schema</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_TryMarkAsInProgressAbortsIf">TryMarkAsInProgressAbortsIf</a> {
    <b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
    <b>aborts_if</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes
        == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>" && !<b>exists</b>&lt;<a href="timestamp.md#0x1_timestamp_CurrentTimeMicroseconds">timestamp::CurrentTimeMicroseconds</a>&gt;(@aptos_framework);
}
</code></pre>



<a id="@Specification_1_start_time_secs"></a>

### Function `start_time_secs`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_start_time_secs">start_time_secs</a>(): u64
</code></pre>




<pre><code><b>aborts_if</b> !<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
<b>include</b>  <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes
    == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>" ==&gt;
 <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_UnpackAbortsIf">copyable_any::UnpackAbortsIf</a>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a>&gt; {
    x:  <b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant
};
<b>aborts_if</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(<b>global</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework).variant).bytes
    != b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>";
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
