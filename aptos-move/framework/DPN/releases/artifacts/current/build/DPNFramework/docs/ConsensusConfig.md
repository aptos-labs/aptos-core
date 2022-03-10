
<a name="0x1_ConsensusConfig"></a>

# Module `0x1::ConsensusConfig`

Maintains the consensus config for the Diem blockchain. The config is stored in a
Reconfiguration, and may be updated by Diem root.


-  [Struct `ConsensusConfig`](#0x1_ConsensusConfig_ConsensusConfig)
-  [Function `initialize`](#0x1_ConsensusConfig_initialize)
-  [Function `set`](#0x1_ConsensusConfig_set)
-  [Module Specification](#@Module_Specification_0)
    -  [Access Control](#@Access_Control_1)


<pre><code><b>use</b> <a href="Reconfiguration.md#0x1_Reconfiguration">0x1::Reconfiguration</a>;
<b>use</b> <a href="Roles.md#0x1_Roles">0x1::Roles</a>;
<b>use</b> <a href="../../../../../../../DPN/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector">0x1::Vector</a>;
</code></pre>



<a name="0x1_ConsensusConfig_ConsensusConfig"></a>

## Struct `ConsensusConfig`



<pre><code><b>struct</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> <b>has</b> <b>copy</b>, drop, store
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

<a name="0x1_ConsensusConfig_initialize"></a>

## Function `initialize`

Publishes the ConsensusConfig config.


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_initialize">initialize</a>(dr_account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_initialize">initialize</a>(dr_account: &signer) {
    <a href="Roles.md#0x1_Roles_assert_diem_root">Roles::assert_diem_root</a>(dr_account);
    <a href="Reconfiguration.md#0x1_Reconfiguration_publish_new_config">Reconfiguration::publish_new_config</a>(dr_account, <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> { config: <a href="../../../../../../../DPN/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>() });
}
</code></pre>



</details>

<details>
<summary>Specification</summary>


Must abort if the signer does not have the DiemRoot role [[H12]][PERMISSION].


<pre><code><b>include</b> <a href="Roles.md#0x1_Roles_AbortsIfNotDiemRoot">Roles::AbortsIfNotDiemRoot</a>{account: dr_account};
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_PublishNewConfigAbortsIf">Reconfiguration::PublishNewConfigAbortsIf</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;;
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_PublishNewConfigEnsures">Reconfiguration::PublishNewConfigEnsures</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;{
    payload: <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> { config: <a href="../../../../../../../DPN/releases/artifacts/current/build/MoveStdlib/docs/Vector.md#0x1_Vector_empty">Vector::empty</a>() }
};
</code></pre>



</details>

<a name="0x1_ConsensusConfig_set"></a>

## Function `set`

Allows Diem root to update the config.


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_set">set</a>(dr_account: &signer, config: vector&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_set">set</a>(dr_account: &signer, config: vector&lt;u8&gt;) {
    <a href="Roles.md#0x1_Roles_assert_diem_root">Roles::assert_diem_root</a>(dr_account);

    <a href="Reconfiguration.md#0x1_Reconfiguration_set">Reconfiguration::set</a>(
        dr_account,
        <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> { config }
    );
}
</code></pre>



</details>

<details>
<summary>Specification</summary>


Must abort if the signer does not have the DiemRoot role [[H12]][PERMISSION].


<pre><code><b>include</b> <a href="Roles.md#0x1_Roles_AbortsIfNotDiemRoot">Roles::AbortsIfNotDiemRoot</a>{account: dr_account};
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_SetAbortsIf">Reconfiguration::SetAbortsIf</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;{account: dr_account};
<b>include</b> <a href="Reconfiguration.md#0x1_Reconfiguration_SetEnsures">Reconfiguration::SetEnsures</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;{payload: <a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a> { config }};
</code></pre>



</details>

<a name="@Module_Specification_0"></a>

## Module Specification



<a name="@Access_Control_1"></a>

### Access Control

The permission "UpdateConsensusConfig" is granted to DiemRoot [[H12]][PERMISSION].


<pre><code><b>invariant</b> [suspendable] <b>forall</b> addr: <b>address</b>
    <b>where</b> <b>exists</b>&lt;<a href="Reconfiguration.md#0x1_Reconfiguration">Reconfiguration</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;&gt;(addr): addr == @DiemRoot;
<b>invariant</b> <b>update</b> [suspendable] <b>old</b>(<a href="Reconfiguration.md#0x1_Reconfiguration_spec_is_published">Reconfiguration::spec_is_published</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;())
    && <a href="Reconfiguration.md#0x1_Reconfiguration_spec_is_published">Reconfiguration::spec_is_published</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;()
    && <b>old</b>(<a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;()) != <a href="Reconfiguration.md#0x1_Reconfiguration_get">Reconfiguration::get</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;()
        ==&gt; <a href="Roles.md#0x1_Roles_spec_signed_by_diem_root_role">Roles::spec_signed_by_diem_root_role</a>();
</code></pre>


Only "set" can modify the ConsensusConfig config [[H12]][PERMISSION]


<a name="0x1_ConsensusConfig_ConsensusConfigRemainsSame"></a>


<pre><code><b>schema</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_ConsensusConfigRemainsSame">ConsensusConfigRemainsSame</a> {
    <b>ensures</b> <b>old</b>(<a href="Reconfiguration.md#0x1_Reconfiguration_spec_is_published">Reconfiguration::spec_is_published</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;()) ==&gt;
        <b>global</b>&lt;<a href="Reconfiguration.md#0x1_Reconfiguration">Reconfiguration</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;&gt;(@DiemRoot) ==
            <b>old</b>(<b>global</b>&lt;<a href="Reconfiguration.md#0x1_Reconfiguration">Reconfiguration</a>&lt;<a href="ConsensusConfig.md#0x1_ConsensusConfig">ConsensusConfig</a>&gt;&gt;(@DiemRoot));
}
</code></pre>




<pre><code><b>apply</b> <a href="ConsensusConfig.md#0x1_ConsensusConfig_ConsensusConfigRemainsSame">ConsensusConfigRemainsSame</a> <b>to</b> * <b>except</b> set;
</code></pre>
