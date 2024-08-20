
<a id="0x1_mpc"></a>

# Module `0x1::mpc`



-  [Struct `SharedSecretState`](#0x1_mpc_SharedSecretState)
-  [Struct `TaskSpec`](#0x1_mpc_TaskSpec)
-  [Struct `TaskState`](#0x1_mpc_TaskState)
-  [Resource `State`](#0x1_mpc_State)
-  [Struct `MPCEvent`](#0x1_mpc_MPCEvent)
-  [Struct `MPCEventReconfigStart`](#0x1_mpc_MPCEventReconfigStart)
-  [Struct `MPCEventStateUpdated`](#0x1_mpc_MPCEventStateUpdated)
-  [Resource `FeatureEnabledFlag`](#0x1_mpc_FeatureEnabledFlag)
-  [Function `initialize`](#0x1_mpc_initialize)
-  [Function `on_async_reconfig_start`](#0x1_mpc_on_async_reconfig_start)
-  [Function `ready_for_next_epoch`](#0x1_mpc_ready_for_next_epoch)
-  [Function `on_new_epoch`](#0x1_mpc_on_new_epoch)
-  [Function `raise_by_secret`](#0x1_mpc_raise_by_secret)
-  [Function `get_result`](#0x1_mpc_get_result)
-  [Function `publish_reconfig_work_result`](#0x1_mpc_publish_reconfig_work_result)
-  [Function `publish_task_result`](#0x1_mpc_publish_task_result)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="next_validator_set.md#0x1_next_validator_set">0x1::next_validator_set</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="reconfiguration.md#0x1_reconfiguration">0x1::reconfiguration</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;
</code></pre>



<a id="0x1_mpc_SharedSecretState"></a>

## Struct `SharedSecretState`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_SharedSecretState">SharedSecretState</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>transcript_for_cur_epoch: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>transcript_for_next_epoch: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_TaskSpec"></a>

## Struct `TaskSpec`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_TaskSpec">TaskSpec</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>group_element: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>secret_idx: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_TaskState"></a>

## Struct `TaskState`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_TaskState">TaskState</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task: <a href="mpc.md#0x1_mpc_TaskSpec">mpc::TaskSpec</a></code>
</dt>
<dd>

</dd>
<dt>
<code>result: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_State"></a>

## Resource `State`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_State">State</a> <b>has</b> <b>copy</b>, drop, store, key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>shared_secrets: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="mpc.md#0x1_mpc_SharedSecretState">mpc::SharedSecretState</a>&gt;</code>
</dt>
<dd>
 Currently only has 1 secret: the main secret.
</dd>
<dt>
<code>tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="mpc.md#0x1_mpc_TaskState">mpc::TaskState</a>&gt;</code>
</dt>
<dd>
 The user request queue.
 mpc todo: scale with Table/BigVector.
</dd>
</dl>


</details>

<a id="0x1_mpc_MPCEvent"></a>

## Struct `MPCEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="mpc.md#0x1_mpc_MPCEvent">MPCEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_MPCEventReconfigStart"></a>

## Struct `MPCEventReconfigStart`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_MPCEventReconfigStart">MPCEventReconfigStart</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="next_validator_set.md#0x1_next_validator_set">next_validator_set</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="validator_consensus_info.md#0x1_validator_consensus_info_ValidatorConsensusInfo">validator_consensus_info::ValidatorConsensusInfo</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_MPCEventStateUpdated"></a>

## Struct `MPCEventStateUpdated`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_MPCEventStateUpdated">MPCEventStateUpdated</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_state: <a href="mpc.md#0x1_mpc_State">mpc::State</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_FeatureEnabledFlag"></a>

## Resource `FeatureEnabledFlag`

This resource exists under 0x1 iff MPC is enabled.


<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_FeatureEnabledFlag">FeatureEnabledFlag</a> <b>has</b> key
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

<a id="0x1_mpc_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_initialize">mpc::initialize</a>: begin"));
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework)) {
        <b>let</b> state = <a href="mpc.md#0x1_mpc_State">State</a> {
            shared_secrets: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
            tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        };
        <b>move_to</b>(framework, state);
        <b>move_to</b>(framework, <a href="mpc.md#0x1_mpc_FeatureEnabledFlag">FeatureEnabledFlag</a> {}); //<a href="mpc.md#0x1_mpc">mpc</a> todo: this needs <b>to</b> be pulled out <b>as</b> part of mpc_config, just like <a href="randomness_config.md#0x1_randomness_config">randomness_config</a>.
    };
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_initialize">mpc::initialize</a>: end"));
}
</code></pre>



</details>

<a id="0x1_mpc_on_async_reconfig_start"></a>

## Function `on_async_reconfig_start`



<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_on_async_reconfig_start">on_async_reconfig_start</a>()
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_on_async_reconfig_start">on_async_reconfig_start</a>() {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_on_async_reconfig_start">mpc::on_async_reconfig_start</a>: begin"));
    <b>if</b> (<b>exists</b>&lt;<a href="mpc.md#0x1_mpc_FeatureEnabledFlag">FeatureEnabledFlag</a>&gt;(@aptos_framework)) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_on_async_reconfig_start">mpc::on_async_reconfig_start</a>: emitting <a href="mpc.md#0x1_mpc">mpc</a> <a href="event.md#0x1_event">event</a>"));
        <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="mpc.md#0x1_mpc_MPCEventReconfigStart">MPCEventReconfigStart</a> {
            epoch: <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>(),
            <a href="next_validator_set.md#0x1_next_validator_set">next_validator_set</a>: <a href="next_validator_set.md#0x1_next_validator_set_load">next_validator_set::load</a>(),
        };
        emit(<a href="mpc.md#0x1_mpc_MPCEvent">MPCEvent</a> { variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="event.md#0x1_event">event</a>)});
    };
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_on_async_reconfig_start">mpc::on_async_reconfig_start</a>: end"));
}
</code></pre>



</details>

<a id="0x1_mpc_ready_for_next_epoch"></a>

## Function `ready_for_next_epoch`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="mpc.md#0x1_mpc_ready_for_next_epoch">ready_for_next_epoch</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="mpc.md#0x1_mpc_ready_for_next_epoch">ready_for_next_epoch</a>(): bool <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_ready_for_next_epoch">mpc::ready_for_next_epoch</a>: begin"));
    <b>if</b> (!<b>exists</b>&lt;<a href="mpc.md#0x1_mpc_FeatureEnabledFlag">FeatureEnabledFlag</a>&gt;(@aptos_framework)) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_ready_for_next_epoch">mpc::ready_for_next_epoch</a>: end, <a href="mpc.md#0x1_mpc">mpc</a> ready 0"));
        <b>return</b> <b>true</b>
    };

    <b>if</b> (!<b>exists</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework)) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_ready_for_next_epoch">mpc::ready_for_next_epoch</a>: end, <a href="mpc.md#0x1_mpc">mpc</a> not ready 1"));
        <b>return</b> <b>false</b>
    };

    <b>let</b> state = <b>borrow_global</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework);
    <b>let</b> num_secrets = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&state.shared_secrets);
    <b>if</b> (num_secrets == 0) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_ready_for_next_epoch">mpc::ready_for_next_epoch</a>: end, <a href="mpc.md#0x1_mpc">mpc</a> not ready 2"));
        <b>return</b> <b>false</b>
    };

    <b>let</b> secret_state = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&state.shared_secrets, 0);
    <b>let</b> maybe_trx = &secret_state.transcript_for_next_epoch;
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(maybe_trx)) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_ready_for_next_epoch">mpc::ready_for_next_epoch</a>: end, <a href="mpc.md#0x1_mpc">mpc</a> not ready 3"));
        <b>return</b> <b>false</b>
    };

    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_ready_for_next_epoch">mpc::ready_for_next_epoch</a>: end, <a href="mpc.md#0x1_mpc">mpc</a> ready 4"));
    <b>true</b>
}
</code></pre>



</details>

<a id="0x1_mpc_on_new_epoch"></a>

## Function `on_new_epoch`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="mpc.md#0x1_mpc_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="mpc.md#0x1_mpc_on_new_epoch">on_new_epoch</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_on_new_epoch">mpc::on_new_epoch</a>: begin"));
    <a href="mpc.md#0x1_mpc_initialize">initialize</a>(framework);
    //<a href="mpc.md#0x1_mpc">mpc</a> todo: should clean up <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> in-progress session states.
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&state.shared_secrets) &gt;= 1) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_on_new_epoch">mpc::on_new_epoch</a>: found main secret"));
        <b>let</b> main_secret_state = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> state.shared_secrets, 0);
        main_secret_state.transcript_for_cur_epoch = main_secret_state.transcript_for_next_epoch;
        main_secret_state.transcript_for_next_epoch = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    };
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_on_new_epoch">mpc::on_new_epoch</a>: end"));
}
</code></pre>



</details>

<a id="0x1_mpc_raise_by_secret"></a>

## Function `raise_by_secret`



<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_raise_by_secret">raise_by_secret</a>(group_element: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secret_idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_raise_by_secret">raise_by_secret</a>(group_element: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secret_idx: u64): u64 <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    //<a href="mpc.md#0x1_mpc">mpc</a> todo: validate group_element
    <b>let</b> task_spec = <a href="mpc.md#0x1_mpc_TaskSpec">TaskSpec</a> {
        group_element,
        secret_idx
    };

    <b>let</b> task_state = <a href="mpc.md#0x1_mpc_TaskState">TaskState</a> {
        task: task_spec,
        result: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    };
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework);
    <b>let</b> task_idx = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&state.tasks);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> state.tasks, task_state);

    <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="mpc.md#0x1_mpc_MPCEventStateUpdated">MPCEventStateUpdated</a> {
        epoch: <a href="reconfiguration.md#0x1_reconfiguration_current_epoch">reconfiguration::current_epoch</a>(),
        new_state: *state,
    };
    emit(<a href="mpc.md#0x1_mpc_MPCEvent">MPCEvent</a> { variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="event.md#0x1_event">event</a>)});

    task_idx
}
</code></pre>



</details>

<a id="0x1_mpc_get_result"></a>

## Function `get_result`

Used by user contracts to get the result.


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_get_result">get_result</a>(task_idx: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_get_result">get_result</a>(task_idx: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework).tasks, task_idx).result
}
</code></pre>



</details>

<a id="0x1_mpc_publish_reconfig_work_result"></a>

## Function `publish_reconfig_work_result`

When a MPC task is done, this is invoked by validator transactions.


<pre><code><b>fun</b> <a href="mpc.md#0x1_mpc_publish_reconfig_work_result">publish_reconfig_work_result</a>(trx: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="mpc.md#0x1_mpc_publish_reconfig_work_result">publish_reconfig_work_result</a>(trx: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_publish_reconfig_work_result">mpc::publish_reconfig_work_result</a>: begin"));
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&state.shared_secrets) == 0) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_publish_reconfig_work_result">mpc::publish_reconfig_work_result</a>: we will have a new secret."));
        <b>let</b> secret_state = <a href="mpc.md#0x1_mpc_SharedSecretState">SharedSecretState</a> {
            transcript_for_next_epoch: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
            transcript_for_cur_epoch: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
        };
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> state.shared_secrets, secret_state);
    };
    <b>let</b> secret_state = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> state.shared_secrets, 0);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&secret_state.transcript_for_next_epoch)) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_publish_reconfig_work_result">mpc::publish_reconfig_work_result</a>: <b>apply</b>"));
        secret_state.transcript_for_next_epoch = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(trx);
    };
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_publish_reconfig_work_result">mpc::publish_reconfig_work_result</a>: end"));
}
</code></pre>



</details>

<a id="0x1_mpc_publish_task_result"></a>

## Function `publish_task_result`



<pre><code><b>fun</b> <a href="mpc.md#0x1_mpc_publish_task_result">publish_task_result</a>(idx: u64, result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="mpc.md#0x1_mpc_publish_task_result">publish_task_result</a>(idx: u64, result: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_publish_task_result">mpc::publish_task_result</a>: begin"));
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework);
    <b>let</b> task_state = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> state.tasks, idx);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&task_state.result)) {
        <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_publish_task_result">mpc::publish_task_result</a>: <b>apply</b>"));
        task_state.result = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(result);
    };
    <a href="../../aptos-stdlib/doc/debug.md#0x1_debug_print">debug::print</a>(&utf8(b"0720 - <a href="mpc.md#0x1_mpc_publish_task_result">mpc::publish_task_result</a>: end"));
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
