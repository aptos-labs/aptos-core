
<a name="0x1_ExperimentalParallelExecutionConfig"></a>

# Module `0x1::ExperimentalParallelExecutionConfig`



-  [Struct `ExperimentalParallelExecutionConfig`](#0x1_ExperimentalParallelExecutionConfig_ExperimentalParallelExecutionConfig)
-  [Function `initialize_parallel_execution`](#0x1_ExperimentalParallelExecutionConfig_initialize_parallel_execution)
-  [Function `enable_parallel_execution_with_config`](#0x1_ExperimentalParallelExecutionConfig_enable_parallel_execution_with_config)
-  [Function `disable_parallel_execution`](#0x1_ExperimentalParallelExecutionConfig_disable_parallel_execution)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig">0x1::ParallelExecutionConfig</a>;
</code></pre>



<a name="0x1_ExperimentalParallelExecutionConfig_ExperimentalParallelExecutionConfig"></a>

## Struct `ExperimentalParallelExecutionConfig`



<pre><code><b>struct</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig">ExperimentalParallelExecutionConfig</a> <b>has</b> drop
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

<a name="0x1_ExperimentalParallelExecutionConfig_initialize_parallel_execution"></a>

## Function `initialize_parallel_execution`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig_initialize_parallel_execution">initialize_parallel_execution</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig_initialize_parallel_execution">initialize_parallel_execution</a>(
    account: &signer,
) {
    <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_initialize_parallel_execution">ParallelExecutionConfig::initialize_parallel_execution</a>&lt;<a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig">ExperimentalParallelExecutionConfig</a>&gt;(account);
    <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_create">Capability::create</a>&lt;<a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig">ExperimentalParallelExecutionConfig</a>&gt;(
        account,
        &<a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig">ExperimentalParallelExecutionConfig</a> {}
    );
}
</code></pre>



</details>

<a name="0x1_ExperimentalParallelExecutionConfig_enable_parallel_execution_with_config"></a>

## Function `enable_parallel_execution_with_config`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig_enable_parallel_execution_with_config">enable_parallel_execution_with_config</a>(account: &signer, read_write_inference_result: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig_enable_parallel_execution_with_config">enable_parallel_execution_with_config</a>(
    account: &signer,
    read_write_inference_result: vector&lt;u8&gt;,
) {
    <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_enable_parallel_execution_with_config">ParallelExecutionConfig::enable_parallel_execution_with_config</a>(
        read_write_inference_result,
        &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig">ExperimentalParallelExecutionConfig</a> {}),
    );
}
</code></pre>



</details>

<a name="0x1_ExperimentalParallelExecutionConfig_disable_parallel_execution"></a>

## Function `disable_parallel_execution`



<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig_disable_parallel_execution">disable_parallel_execution</a>(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig_disable_parallel_execution">disable_parallel_execution</a>(account: &signer) {
    <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_disable_parallel_execution">ParallelExecutionConfig::disable_parallel_execution</a>(
        &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="ExperimentalParallelExecutionConfig.md#0x1_ExperimentalParallelExecutionConfig">ExperimentalParallelExecutionConfig</a> {}),
    );
}
</code></pre>



</details>
