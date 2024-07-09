
<a id="0x1_mpc"></a>

# Module `0x1::mpc`



-  [Struct `SharedSecret`](#0x1_mpc_SharedSecret)
-  [Struct `TaskSpec`](#0x1_mpc_TaskSpec)
-  [Struct `TaskState`](#0x1_mpc_TaskState)
-  [Struct `TaskRaiseBySecret`](#0x1_mpc_TaskRaiseBySecret)
-  [Resource `State`](#0x1_mpc_State)
-  [Struct `NewTaskEvent`](#0x1_mpc_NewTaskEvent)
-  [Struct `TaskCompletedEvent`](#0x1_mpc_TaskCompletedEvent)
-  [Function `raise_by_secret`](#0x1_mpc_raise_by_secret)
-  [Function `fulfill_task`](#0x1_mpc_fulfill_task)
-  [Function `get_result`](#0x1_mpc_get_result)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="0x1_mpc_SharedSecret"></a>

## Struct `SharedSecret`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_SharedSecret">SharedSecret</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>transcript_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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
<code>variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_TaskState"></a>

## Struct `TaskState`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_TaskState">TaskState</a> <b>has</b> store
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

<a id="0x1_mpc_TaskRaiseBySecret"></a>

## Struct `TaskRaiseBySecret`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_TaskRaiseBySecret">TaskRaiseBySecret</a> <b>has</b> <b>copy</b>, drop, store
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

<a id="0x1_mpc_State"></a>

## Resource `State`



<pre><code><b>struct</b> <a href="mpc.md#0x1_mpc_State">State</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>shared_secrets: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="mpc.md#0x1_mpc_SharedSecret">mpc::SharedSecret</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>tasks: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="mpc.md#0x1_mpc_TaskState">mpc::TaskState</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_NewTaskEvent"></a>

## Struct `NewTaskEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="mpc.md#0x1_mpc_NewTaskEvent">NewTaskEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_idx: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>task_spec: <a href="mpc.md#0x1_mpc_TaskSpec">mpc::TaskSpec</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_mpc_TaskCompletedEvent"></a>

## Struct `TaskCompletedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="mpc.md#0x1_mpc_TaskCompletedEvent">TaskCompletedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>task_idx: u64</code>
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

<a id="0x1_mpc_raise_by_secret"></a>

## Function `raise_by_secret`



<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_raise_by_secret">raise_by_secret</a>(group_element: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secret_idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_raise_by_secret">raise_by_secret</a>(group_element: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, secret_idx: u64): u64 <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <b>let</b> task_spec = <a href="mpc.md#0x1_mpc_TaskSpec">TaskSpec</a> {
        variant: <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_pack">copyable_any::pack</a>(<a href="mpc.md#0x1_mpc_TaskRaiseBySecret">TaskRaiseBySecret</a> {
            group_element,
            secret_idx
        }),
    };

    <b>let</b> task_state = <a href="mpc.md#0x1_mpc_TaskState">TaskState</a> {
        task: task_spec,
        result: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>(),
    };
    <b>let</b> task_list = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework).tasks;
    <b>let</b> task_idx = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(task_list);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(task_list, task_state);

    <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="mpc.md#0x1_mpc_NewTaskEvent">NewTaskEvent</a> {
        task_idx,
        task_spec
    };
    emit(<a href="event.md#0x1_event">event</a>);

    task_idx
}
</code></pre>



</details>

<a id="0x1_mpc_fulfill_task"></a>

## Function `fulfill_task`

When a MPC task is done, this is invoked by validator transactions.


<pre><code><b>fun</b> <a href="mpc.md#0x1_mpc_fulfill_task">fulfill_task</a>(task_idx: u64, result: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="mpc.md#0x1_mpc_fulfill_task">fulfill_task</a>(task_idx: u64, result: Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework).tasks, task_idx).result = result;
    <b>let</b> <a href="event.md#0x1_event">event</a> = <a href="mpc.md#0x1_mpc_TaskCompletedEvent">TaskCompletedEvent</a> {
        task_idx,
        result,
    };
    emit(<a href="event.md#0x1_event">event</a>);
}
</code></pre>



</details>

<a id="0x1_mpc_get_result"></a>

## Function `get_result`

Used by user contract to get the result.


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_get_result">get_result</a>(task_idx: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="mpc.md#0x1_mpc_get_result">get_result</a>(task_idx: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="mpc.md#0x1_mpc_State">State</a> {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="mpc.md#0x1_mpc_State">State</a>&gt;(@aptos_framework).tasks, task_idx).result
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
