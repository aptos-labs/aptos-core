
<a name="0x1_ConsensusConfig"></a>

# Module `0x1::ConsensusConfig`

Maintains the consensus config for the blockchain. The config is stored in a
Reconfiguration, and may be updated by root.


-  [Resource `ConsensusConfigChainMarker`](#0x1_ConsensusConfig_ConsensusConfigChainMarker)
-  [Resource `ConsensusConfig`](#0x1_ConsensusConfig_ConsensusConfig)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_ConsensusConfig_initialize)
-  [Function `set`](#0x1_ConsensusConfig_set)


<pre><code><b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="Reconfiguration.md#0x1_Reconfiguration">0x1::Reconfiguration</a>;
<b>use</b> <a href="SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
<b>use</b> <a href="Timestamp.md#0x1_Timestamp">0x1::Timestamp</a>;
<b>use</b> <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_ConsensusConfig_ConsensusConfigChainMarker"></a>

## Resource `ConsensusConfigChainMarker`

Marker to be stored under @CoreResources during genesis


<pre><code><b>struct</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_ConsensusConfigChainMarker">ConsensusConfigChainMarker</a>&lt;T&gt; <b>has</b> key
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

<a name="0x1_ConsensusConfig_ConsensusConfig"></a>

## Resource `ConsensusConfig`



<pre><code><b>struct</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>config: vector&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_ConsensusConfig_ECONFIG"></a>

Error with config


<pre><code><b>const</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_ECONFIG">ECONFIG</a>: u64 = 1;
</code></pre>



<a name="0x1_ConsensusConfig_ECHAIN_MARKER"></a>

Error with chain marker


<pre><code><b>const</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_ECHAIN_MARKER">ECHAIN_MARKER</a>: u64 = 0;
</code></pre>



<a name="0x1_ConsensusConfig_initialize"></a>

## Function `initialize`

Publishes the ConsensusConfig config.


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_initialize">initialize</a>&lt;T&gt;(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_initialize">initialize</a>&lt;T&gt;(account: &signer) {
    <a href="Timestamp.md#0x1_Timestamp_assert_genesis">Timestamp::assert_genesis</a>();
    <a href="SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(account);
    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig_ConsensusConfigChainMarker">ConsensusConfigChainMarker</a>&lt;T&gt;&gt;(@CoreResources),
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="ConsensusConfig.md#0x1_ConsensusConfig_ECHAIN_MARKER">ECHAIN_MARKER</a>)
    );

    <b>assert</b>!(
        !<b>exists</b>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;(@CoreResources),
        <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_already_published">Errors::already_published</a>(<a href="ConsensusConfig.md#0x1_ConsensusConfig_ECONFIG">ECONFIG</a>)
    );
    <b>move_to</b>(account, <a href="ConsensusConfig.md#0x1_ConsensusConfig_ConsensusConfigChainMarker">ConsensusConfigChainMarker</a>&lt;T&gt;{});
    <b>move_to</b>(account, <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> { config: <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>() });
}
</code></pre>



</details>

<a name="0x1_ConsensusConfig_set"></a>

## Function `set`

Update the config.


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_set">set</a>&lt;T&gt;(config: vector&lt;u8&gt;, _cap: &<a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_Cap">Capability::Cap</a>&lt;T&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_set">set</a>&lt;T&gt;(config: vector&lt;u8&gt;, _cap: &Cap&lt;T&gt;) <b>acquires</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig_ConsensusConfigChainMarker">ConsensusConfigChainMarker</a>&lt;T&gt;&gt;(@CoreResources), <a href="../../../../../../../experimental/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_not_published">Errors::not_published</a>(<a href="ConsensusConfig.md#0x1_ConsensusConfig_ECHAIN_MARKER">ECHAIN_MARKER</a>));
    <b>let</b> config_ref = &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;(@CoreResources).config;
    *config_ref = config;
    <a href="Reconfiguration.md#0x1_Reconfiguration_reconfigure">Reconfiguration::reconfigure</a>();
}
</code></pre>



</details>


[//]: # ("File containing references which can be used from documentation")
[ACCESS_CONTROL]: https://github.com/diem/dip/blob/main/dips/dip-2.md
[ROLE]: https://github.com/diem/dip/blob/main/dips/dip-2.md#roles
[PERMISSION]: https://github.com/diem/dip/blob/main/dips/dip-2.md#permissions
