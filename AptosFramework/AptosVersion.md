
<a name="0x1_AptosVersion"></a>

# Module `0x1::AptosVersion`



-  [Function `initialize`](#0x1_AptosVersion_initialize)
-  [Function `set_version`](#0x1_AptosVersion_set_version)


<pre><code><b>use</b> <a href="../MoveStdlib/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
<b>use</b> <a href="../CoreFramework/Version.md#0x1_Version">0x1::Version</a>;
</code></pre>



<a name="0x1_AptosVersion_initialize"></a>

## Function `initialize`

Publishes the Version config.


<pre><code><b>public</b> <b>fun</b> <a href="AptosVersion.md#0x1_AptosVersion_initialize">initialize</a>(account: &signer, initial_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosVersion.md#0x1_AptosVersion_initialize">initialize</a>(account: &signer, initial_version: u64) {
    <a href="../CoreFramework/Version.md#0x1_Version_initialize">Version::initialize</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(account, initial_version);
}
</code></pre>



</details>

<a name="0x1_AptosVersion_set_version"></a>

## Function `set_version`

Updates the major version to a larger version.


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosVersion.md#0x1_AptosVersion_set_version">set_version</a>(account: signer, major: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>script</b>) <b>fun</b> <a href="AptosVersion.md#0x1_AptosVersion_set_version">set_version</a>(account: signer, major: u64) {
    <a href="../CoreFramework/Version.md#0x1_Version_set">Version::set</a>&lt;<a href="Marker.md#0x1_Marker_ChainMarker">Marker::ChainMarker</a>&gt;(
        major,
        &<a href="../MoveStdlib/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(&account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>()),
    );
}
</code></pre>



</details>
