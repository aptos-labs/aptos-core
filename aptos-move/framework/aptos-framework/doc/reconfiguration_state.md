
<a id="0x1_reconfiguration_state"></a>

# Module `0x1::reconfiguration_state`

Reconfiguration meta-state resources and util functions.

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



<a id="0x1_reconfiguration_state_is_initialized"></a>

## Function `is_initialized`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_initialized">is_initialized</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_is_initialized">is_initialized</a>(): bool {
    <b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)
}
</code></pre>



</details>

<a id="0x1_reconfiguration_state_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_initialize">initialize</a>(fx: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
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

<a id="0x1_reconfiguration_state_on_reconfig_start"></a>

## Function `on_reconfig_start`

Called at the beginning of a reconfiguration (either immediate or async)
to mark the reconfiguration state "in progress" if it is currently "stopped".

Also record the current time as the reconfiguration start time. (Some module, e.g., <code><a href="stake.md#0x1_stake">stake</a>.<b>move</b></code>, needs this info).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">on_reconfig_start</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_start">on_reconfig_start</a>() <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) {
        <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
        <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&state.variant));
        <b>if</b> (variant_type_name == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">0x1::reconfiguration_state::StateInactive</a>") {
            state.variant = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">StateActive</a> {
                start_time_secs: <a href="timestamp.md#0x1_timestamp_now_seconds">timestamp::now_seconds</a>()
            });
        }
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

<a id="0x1_reconfiguration_state_on_reconfig_finish"></a>

## Function `on_reconfig_finish`

Called at the end of every reconfiguration to mark the state as "stopped".
Abort if the current state is not "in progress".


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_finish">on_reconfig_finish</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_on_reconfig_finish">on_reconfig_finish</a>() <b>acquires</b> <a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a> {
    <b>if</b> (<b>exists</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework)) {
        <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="reconfiguration_state.md#0x1_reconfiguration_state_State">State</a>&gt;(@aptos_framework);
        <b>let</b> variant_type_name = *<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_type_name">copyable_any::type_name</a>(&state.variant));
        <b>if</b> (variant_type_name == b"<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateActive">0x1::reconfiguration_state::StateActive</a>") {
            state.variant = <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_StateInactive">StateInactive</a> {});
        } <b>else</b> {
            <b>abort</b>(<a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_state">error::invalid_state</a>(<a href="reconfiguration_state.md#0x1_reconfiguration_state_ERECONFIG_NOT_IN_PROGRESS">ERECONFIG_NOT_IN_PROGRESS</a>))
        }
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
