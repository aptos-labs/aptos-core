
<a name="0x1_ParallelExecutionConfig"></a>

# Module `0x1::ParallelExecutionConfig`

This module defines structs and methods to initialize VM configurations,
including different costs of running the VM.


-  [Struct `ParallelExecutionConfig`](#0x1_ParallelExecutionConfig_ParallelExecutionConfig)
-  [Function `initialize_parallel_execution`](#0x1_ParallelExecutionConfig_initialize_parallel_execution)
-  [Function `enable_parallel_execution_with_config`](#0x1_ParallelExecutionConfig_enable_parallel_execution_with_config)
-  [Function `disable_parallel_execution`](#0x1_ParallelExecutionConfig_disable_parallel_execution)


<pre><code><b>use</b> <a href="DiemConfig.md#0x1_DiemConfig">0x1::DiemConfig</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp">0x1::DiemTimestamp</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
</code></pre>



<a name="0x1_ParallelExecutionConfig_ParallelExecutionConfig"></a>

## Struct `ParallelExecutionConfig`

The struct to hold the read/write set analysis result for the whole Diem Framework.


<pre><code><b>struct</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig">ParallelExecutionConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>read_write_analysis_result: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_Option">Option::Option</a>&lt;vector&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Serialized analysis result for the Diem Framework.
 If this payload is not None, DiemVM will use this config to execute transactions in parallel.
</dd>
</dl>


</details>

<a name="0x1_ParallelExecutionConfig_initialize_parallel_execution"></a>

## Function `initialize_parallel_execution`

Enable parallel execution functionality of DiemVM by setting the read_write_set analysis result.


<pre><code><b>public</b> <b>fun</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_initialize_parallel_execution">initialize_parallel_execution</a>(dr_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_initialize_parallel_execution">initialize_parallel_execution</a>(
    dr_account: &signer,
) {
    // The permission "UpdateVMConfig" is granted <b>to</b> DiemRoot [[H11]][PERMISSION].
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(dr_account);
    <a href="DiemConfig.md#0x1_DiemConfig_publish_new_config">DiemConfig::publish_new_config</a>(
        dr_account,
        <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig">ParallelExecutionConfig</a> {
            read_write_analysis_result: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
        },
    );
}
</code></pre>



</details>

<a name="0x1_ParallelExecutionConfig_enable_parallel_execution_with_config"></a>

## Function `enable_parallel_execution_with_config`



<pre><code><b>public</b> <b>fun</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_enable_parallel_execution_with_config">enable_parallel_execution_with_config</a>(dr_account: &signer, read_write_inference_result: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_enable_parallel_execution_with_config">enable_parallel_execution_with_config</a>(
   dr_account: &signer,
   read_write_inference_result: vector&lt;u8&gt;,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp_assert_operating">DiemTimestamp::assert_operating</a>();
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(dr_account);
    <a href="DiemConfig.md#0x1_DiemConfig_set">DiemConfig::set</a>(dr_account, <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig">ParallelExecutionConfig</a> {
        read_write_analysis_result: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_some">Option::some</a>(read_write_inference_result),
    });
}
</code></pre>



</details>

<a name="0x1_ParallelExecutionConfig_disable_parallel_execution"></a>

## Function `disable_parallel_execution`



<pre><code><b>public</b> <b>fun</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_disable_parallel_execution">disable_parallel_execution</a>(dr_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig_disable_parallel_execution">disable_parallel_execution</a>(
   dr_account: &signer,
) {
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp_assert_operating">DiemTimestamp::assert_operating</a>();
    <a href="../../../../../../../experimental/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(dr_account);
    <a href="DiemConfig.md#0x1_DiemConfig_set">DiemConfig::set</a>(dr_account, <a href="ParallelExecutionConfig.md#0x1_ParallelExecutionConfig">ParallelExecutionConfig</a> {
        read_write_analysis_result: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
    });
}
</code></pre>



</details>
