
<a name="0x1_Marker"></a>

# Module `0x1::Marker`



-  [Struct `ChainMarker`](#0x1_Marker_ChainMarker)
-  [Function `get`](#0x1_Marker_get)
-  [Function `initialize`](#0x1_Marker_initialize)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp">0x1::DiemTimestamp</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses">0x1::SystemAddresses</a>;
</code></pre>



<a name="0x1_Marker_ChainMarker"></a>

## Struct `ChainMarker`



<pre><code><b>struct</b> <a href="Marker.md#0x1_Marker_ChainMarker">ChainMarker</a> <b>has</b> drop
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

<a name="0x1_Marker_get"></a>

## Function `get`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Marker.md#0x1_Marker_get">get</a>(): <a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="Marker.md#0x1_Marker_get">get</a>(): <a href="Marker.md#0x1_Marker_ChainMarker">ChainMarker</a> {
    <a href="Marker.md#0x1_Marker_ChainMarker">ChainMarker</a> {}
}
</code></pre>



</details>

<a name="0x1_Marker_initialize"></a>

## Function `initialize`

Initialize the capability of the marker so friend modules can acquire it for priviledged operations.


<pre><code><b>public</b> <b>fun</b> <a href="Marker.md#0x1_Marker_initialize">initialize</a>(core_resource: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Marker.md#0x1_Marker_initialize">initialize</a>(core_resource: &signer) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/DiemTimestamp.md#0x1_DiemTimestamp_assert_genesis">DiemTimestamp::assert_genesis</a>();
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/DiemCoreFramework/docs/SystemAddresses.md#0x1_SystemAddresses_assert_core_resource">SystemAddresses::assert_core_resource</a>(core_resource);
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_create">Capability::create</a>(core_resource, &<a href="Marker.md#0x1_Marker_get">get</a>());
}
</code></pre>



</details>
