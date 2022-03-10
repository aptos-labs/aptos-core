
<a name="0x1_Version"></a>

# Module `0x1::Version`

Maintains the version number for the Diem blockchain. The version is stored in a
Reconfiguration, and may be updated by Diem root.


-  [Struct `Version`](#0x1_Version_Version)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_Version_initialize)
-  [Function `set`](#0x1_Version_set)
-  [Module Specification](#@Module_Specification_1)
    -  [Initialization](#@Initialization_2)
    -  [Access Control](#@Access_Control_3)
    -  [Other Invariants](#@Other_Invariants_4)


<pre><code><b>use</b> <a href="../../../../../../../DPN/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors">0x1::Errors</a>;
<b>use</b> <a href="Reconfiguration.md#0x1_Reconfiguration">0x1::Reconfiguration</a>;
<b>use</b> <a href="Roles.md#0x1_Roles">0x1::Roles</a>;
<b>use</b> <a href="Timestamp.md#0x1_Timestamp">0x1::Timestamp</a>;
</code></pre>



<a name="0x1_Version_Version"></a>

## Struct `Version`



<pre><code><b>struct</b> <a href="Version.md#0x1_Version">Version</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>major: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="@Constants_0"></a>

## Constants


<a name="0x1_Version_EINVALID_MAJOR_VERSION_NUMBER"></a>

Tried to set an invalid major version for the VM. Major versions must be strictly increasing


<pre><code><b>const</b> <a href="Version.md#0x1_Version_EINVALID_MAJOR_VERSION_NUMBER">EINVALID_MAJOR_VERSION_NUMBER</a>: u64 = 0;
</code></pre>



<a name="0x1_Version_initialize"></a>

## Function `initialize`

Publishes the Version config. Must be called during Genesis.


<pre><code><b>public</b> <b>fun</b> <a href="Version.md#0x1_Version_initialize">initialize</a>(dr_account: &signer, initial_version: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Version.md#0x1_Version_initialize">initialize</a>(dr_account: &signer, initial_version: u64) {
    <a href="Timestamp.md#0x1_Timestamp_assert_genesis">Timestamp::assert_genesis</a>();
    <a href="Roles.md#0x1_Roles_assert_diem_root">Roles::assert_diem_root</a>(dr_account);
    <a href="Reconfiguration.md#0x1_Reconfiguration_publish_new_config">Reconfiguration::publish_new_config</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;(
        dr_account,
        <a href="Version.md#0x1_Version">Version</a> { major: initial_version },
    );
}
</code></pre>



</details>

<details>
<summary>Specification</summary>


Must abort if the signer does not have the DiemRoot role [[H10]][PERMISSION].


<pre><code><b>include</b> <a href="Roles.md#0x1_Roles_AbortsIfNotDiemRoot">Roles::AbortsIfNotDiemRoot</a>{account: dr_account};
<b>include</b> <a href="Timestamp.md#0x1_Timestamp_AbortsIfNotGenesis">Timestamp::AbortsIfNotGenesis</a>;
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_PublishNewConfigAbortsIf">Reconfiguration::PublishNewConfigAbortsIf</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;;
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_PublishNewConfigEnsures">Reconfiguration::PublishNewConfigEnsures</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;{payload: <a href="Version.md#0x1_Version">Version</a> { major: initial_version }};
</code></pre>



</details>

<a name="0x1_Version_set"></a>

## Function `set`

Allows Diem root to update the major version to a larger version.


<pre><code><b>public</b> <b>fun</b> <a href="Version.md#0x1_Version_set">set</a>(dr_account: &signer, major: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="Version.md#0x1_Version_set">set</a>(dr_account: &signer, major: u64) {
    <a href="Timestamp.md#0x1_Timestamp_assert_operating">Timestamp::assert_operating</a>();

    <a href="Roles.md#0x1_Roles_assert_diem_root">Roles::assert_diem_root</a>(dr_account);

    <b>let</b> old_config = <a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;();

    <b>assert</b>!(
        old_config.major &lt; major,
        <a href="../../../../../../../DPN/releases/artifacts/current/build/MoveStdlib/docs/Errors.md#0x1_Errors_invalid_argument">Errors::invalid_argument</a>(<a href="Version.md#0x1_Version_EINVALID_MAJOR_VERSION_NUMBER">EINVALID_MAJOR_VERSION_NUMBER</a>)
    );

    <a href="Reconfiguration.md#0x1_Reconfiguration_set">Reconfiguration::set</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;(
        dr_account,
        <a href="Version.md#0x1_Version">Version</a> { major }
    );
}
</code></pre>



</details>

<details>
<summary>Specification</summary>


Must abort if the signer does not have the DiemRoot role [[H10]][PERMISSION].


<pre><code><b>include</b> <a href="Roles.md#0x1_Roles_AbortsIfNotDiemRoot">Roles::AbortsIfNotDiemRoot</a>{account: dr_account};
<b>include</b> <a href="Timestamp.md#0x1_Timestamp_AbortsIfNotOperating">Timestamp::AbortsIfNotOperating</a>;
<b>aborts_if</b> <a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;().major &gt;= major <b>with</b> Errors::INVALID_ARGUMENT;
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_SetAbortsIf">Reconfiguration::SetAbortsIf</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;{account: dr_account};
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_SetEnsures">Reconfiguration::SetEnsures</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;{payload: <a href="Version.md#0x1_Version">Version</a> { major }};
</code></pre>



</details>

<a name="@Module_Specification_1"></a>

## Module Specification



<a name="@Initialization_2"></a>

### Initialization


After genesis, version is published.


<pre><code><b>invariant</b> [suspendable] <a href="Timestamp.md#0x1_Timestamp_is_operating">Timestamp::is_operating</a>() ==&gt; <a href="Reconfiguration.md#0x1_Reconfiguration_spec_is_published">Reconfiguration::spec_is_published</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;();
</code></pre>



<a name="@Access_Control_3"></a>

### Access Control

The permission "UpdateDiemProtocolVersion" is granted to DiemRoot [[H10]][PERMISSION].


<pre><code><b>invariant</b> [suspendable] <b>forall</b> addr: <b>address</b>
    <b>where</b> <b>exists</b>&lt;<a href="Reconfiguration.md#0x1_Reconfiguration">Reconfiguration</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;&gt;(addr): addr == @DiemRoot;
<b>invariant</b> <b>update</b> [suspendable] <b>old</b>(<a href="Reconfiguration.md#0x1_Reconfiguration_spec_is_published">Reconfiguration::spec_is_published</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;())
    && <a href="Reconfiguration.md#0x1_Reconfiguration_spec_is_published">Reconfiguration::spec_is_published</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;()
    && <b>old</b>(<a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;().major) != <a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;().major
        ==&gt; <a href="Roles.md#0x1_Roles_spec_signed_by_diem_root_role">Roles::spec_signed_by_diem_root_role</a>();
</code></pre>


Only "set" can modify the Version config [[H10]][PERMISSION]


<a name="0x1_Version_VersionRemainsSame"></a>


<pre><code><b>schema</b> <a href="Version.md#0x1_Version_VersionRemainsSame">VersionRemainsSame</a> {
    <b>ensures</b> <b>old</b>(<a href="Reconfiguration.md#0x1_Reconfiguration_spec_is_published">Reconfiguration::spec_is_published</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;()) ==&gt;
        <b>global</b>&lt;<a href="Reconfiguration.md#0x1_Reconfiguration">Reconfiguration</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;&gt;(@DiemRoot) ==
            <b>old</b>(<b>global</b>&lt;<a href="Reconfiguration.md#0x1_Reconfiguration">Reconfiguration</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;&gt;(@DiemRoot));
}
</code></pre>


The permission "UpdateDiemProtocolVersion" is granted to DiemRoot [[H10]][PERMISSION].


<pre><code><b>apply</b> <a href="Version.md#0x1_Version_VersionRemainsSame">VersionRemainsSame</a> <b>to</b> * <b>except</b> set;
</code></pre>



<a name="@Other_Invariants_4"></a>

### Other Invariants


Version number never decreases


<pre><code><b>invariant</b> <b>update</b> [suspendable]
    <a href="Timestamp.md#0x1_Timestamp_is_operating">Timestamp::is_operating</a>() ==&gt;
        (<b>old</b>(<a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;().major) &lt;= <a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="Version.md#0x1_Version">Version</a>&gt;().major);
</code></pre>
