
<a id="0x1_code"></a>

# Module `0x1::code`

This module supports functionality related to code management.


-  [Resource `PackageRegistry`](#0x1_code_PackageRegistry)
-  [Struct `PackageMetadata`](#0x1_code_PackageMetadata)
-  [Struct `PackageDep`](#0x1_code_PackageDep)
-  [Struct `ModuleMetadata`](#0x1_code_ModuleMetadata)
-  [Struct `UpgradePolicy`](#0x1_code_UpgradePolicy)
-  [Struct `PublishPackage`](#0x1_code_PublishPackage)
-  [Struct `AllowedDep`](#0x1_code_AllowedDep)
-  [Constants](#@Constants_0)
-  [Function `upgrade_policy_arbitrary`](#0x1_code_upgrade_policy_arbitrary)
-  [Function `upgrade_policy_compat`](#0x1_code_upgrade_policy_compat)
-  [Function `upgrade_policy_immutable`](#0x1_code_upgrade_policy_immutable)
-  [Function `can_change_upgrade_policy_to`](#0x1_code_can_change_upgrade_policy_to)
-  [Function `initialize`](#0x1_code_initialize)
-  [Function `publish_package`](#0x1_code_publish_package)
-  [Function `freeze_code_object`](#0x1_code_freeze_code_object)
-  [Function `publish_package_txn`](#0x1_code_publish_package_txn)
-  [Function `check_upgradability`](#0x1_code_check_upgradability)
-  [Function `check_coexistence`](#0x1_code_check_coexistence)
-  [Function `check_dependencies`](#0x1_code_check_dependencies)
-  [Function `is_policy_exempted_address`](#0x1_code_is_policy_exempted_address)
-  [Function `get_module_names`](#0x1_code_get_module_names)
-  [Function `request_publish`](#0x1_code_request_publish)
-  [Function `request_publish_with_allowed_deps`](#0x1_code_request_publish_with_allowed_deps)
-  [Specification](#@Specification_1)
    -  [High-level Requirements](#high-level-req)
    -  [Module-level Specification](#module-level-spec)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `publish_package`](#@Specification_1_publish_package)
    -  [Function `freeze_code_object`](#@Specification_1_freeze_code_object)
    -  [Function `publish_package_txn`](#@Specification_1_publish_package_txn)
    -  [Function `check_upgradability`](#@Specification_1_check_upgradability)
    -  [Function `check_coexistence`](#@Specification_1_check_coexistence)
    -  [Function `check_dependencies`](#@Specification_1_check_dependencies)
    -  [Function `get_module_names`](#@Specification_1_get_module_names)
    -  [Function `request_publish`](#@Specification_1_request_publish)
    -  [Function `request_publish_with_allowed_deps`](#@Specification_1_request_publish_with_allowed_deps)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any">0x1::copyable_any</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;<br /><b>use</b> <a href="event.md#0x1_event">0x1::event</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features">0x1::features</a>;<br /><b>use</b> <a href="object.md#0x1_object">0x1::object</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;<br /><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;<br /><b>use</b> <a href="util.md#0x1_util">0x1::util</a>;<br /><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;<br /></code></pre>



<a id="0x1_code_PackageRegistry"></a>

## Resource `PackageRegistry`

The package registry at the given address.


<pre><code><b>struct</b> <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> <b>has</b> drop, store, key<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>packages: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>&gt;</code>
</dt>
<dd>
 Packages installed at this address.
</dd>
</dl>


</details>

<a id="0x1_code_PackageMetadata"></a>

## Struct `PackageMetadata`

Metadata for a package. All byte blobs are represented as base64&#45;of&#45;gzipped&#45;bytes


<pre><code><b>struct</b> <a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Name of this package.
</dd>
<dt>
<code>upgrade_policy: <a href="code.md#0x1_code_UpgradePolicy">code::UpgradePolicy</a></code>
</dt>
<dd>
 The upgrade policy of this package.
</dd>
<dt>
<code>upgrade_number: u64</code>
</dt>
<dd>
 The numbers of times this module has been upgraded. Also serves as the on&#45;chain version.
 This field will be automatically assigned on successful upgrade.
</dd>
<dt>
<code>source_digest: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 The source digest of the sources in the package. This is constructed by first building the
 sha256 of each individual source, than sorting them alphabetically, and sha256 them again.
</dd>
<dt>
<code>manifest: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The package manifest, in the Move.toml format. Gzipped text.
</dd>
<dt>
<code>modules: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_ModuleMetadata">code::ModuleMetadata</a>&gt;</code>
</dt>
<dd>
 The list of modules installed by this package.
</dd>
<dt>
<code>deps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_PackageDep">code::PackageDep</a>&gt;</code>
</dt>
<dd>
 Holds PackageDeps.
</dd>
<dt>
<code>extension: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a>&gt;</code>
</dt>
<dd>
 For future extension
</dd>
</dl>


</details>

<a id="0x1_code_PackageDep"></a>

## Struct `PackageDep`

A dependency to a package published at address


<pre><code><b>struct</b> <a href="code.md#0x1_code_PackageDep">PackageDep</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>package_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_code_ModuleMetadata"></a>

## Struct `ModuleMetadata`

Metadata about a module in a package.


<pre><code><b>struct</b> <a href="code.md#0x1_code_ModuleMetadata">ModuleMetadata</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Name of the module.
</dd>
<dt>
<code>source: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 Source text, gzipped String. Empty if not provided.
</dd>
<dt>
<code>source_map: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 Source map, in compressed BCS. Empty if not provided.
</dd>
<dt>
<code>extension: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/doc/copyable_any.md#0x1_copyable_any_Any">copyable_any::Any</a>&gt;</code>
</dt>
<dd>
 For future extensions.
</dd>
</dl>


</details>

<a id="0x1_code_UpgradePolicy"></a>

## Struct `UpgradePolicy`

Describes an upgrade policy


<pre><code><b>struct</b> <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a> <b>has</b> <b>copy</b>, drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>policy: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_code_PublishPackage"></a>

## Struct `PublishPackage`

Event emitted when code is published to an address.


<pre><code>&#35;[<a href="event.md#0x1_event">event</a>]<br /><b>struct</b> <a href="code.md#0x1_code_PublishPackage">PublishPackage</a> <b>has</b> drop, store<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>code_address: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>is_upgrade: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_code_AllowedDep"></a>

## Struct `AllowedDep`

A helper type for request_publish_with_allowed_deps


<pre><code><b>struct</b> <a href="code.md#0x1_code_AllowedDep">AllowedDep</a> <b>has</b> drop<br /></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>
 Address of the module.
</dd>
<dt>
<code>module_name: <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a></code>
</dt>
<dd>
 Name of the module. If this is the empty string, then this serves as a wildcard for
 all modules from this address. This is used for speeding up dependency checking for packages from
 well&#45;known framework addresses, where we can assume that there are no malicious packages.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_code_ECODE_OBJECT_DOES_NOT_EXIST"></a>

<code>code_object</code> does not exist.


<pre><code><b>const</b> <a href="code.md#0x1_code_ECODE_OBJECT_DOES_NOT_EXIST">ECODE_OBJECT_DOES_NOT_EXIST</a>: u64 &#61; 10;<br /></code></pre>



<a id="0x1_code_EDEP_ARBITRARY_NOT_SAME_ADDRESS"></a>

A dependency to an <code>arbitrary</code> package must be on the same address.


<pre><code><b>const</b> <a href="code.md#0x1_code_EDEP_ARBITRARY_NOT_SAME_ADDRESS">EDEP_ARBITRARY_NOT_SAME_ADDRESS</a>: u64 &#61; 7;<br /></code></pre>



<a id="0x1_code_EDEP_WEAKER_POLICY"></a>

A dependency cannot have a weaker upgrade policy.


<pre><code><b>const</b> <a href="code.md#0x1_code_EDEP_WEAKER_POLICY">EDEP_WEAKER_POLICY</a>: u64 &#61; 6;<br /></code></pre>



<a id="0x1_code_EINCOMPATIBLE_POLICY_DISABLED"></a>

Creating a package with incompatible upgrade policy is disabled.


<pre><code><b>const</b> <a href="code.md#0x1_code_EINCOMPATIBLE_POLICY_DISABLED">EINCOMPATIBLE_POLICY_DISABLED</a>: u64 &#61; 8;<br /></code></pre>



<a id="0x1_code_EMODULE_MISSING"></a>

Cannot delete a module that was published in the same package


<pre><code><b>const</b> <a href="code.md#0x1_code_EMODULE_MISSING">EMODULE_MISSING</a>: u64 &#61; 4;<br /></code></pre>



<a id="0x1_code_EMODULE_NAME_CLASH"></a>

Package contains duplicate module names with existing modules publised in other packages on this address


<pre><code><b>const</b> <a href="code.md#0x1_code_EMODULE_NAME_CLASH">EMODULE_NAME_CLASH</a>: u64 &#61; 1;<br /></code></pre>



<a id="0x1_code_ENOT_PACKAGE_OWNER"></a>

Not the owner of the package registry.


<pre><code><b>const</b> <a href="code.md#0x1_code_ENOT_PACKAGE_OWNER">ENOT_PACKAGE_OWNER</a>: u64 &#61; 9;<br /></code></pre>



<a id="0x1_code_EPACKAGE_DEP_MISSING"></a>

Dependency could not be resolved to any published package.


<pre><code><b>const</b> <a href="code.md#0x1_code_EPACKAGE_DEP_MISSING">EPACKAGE_DEP_MISSING</a>: u64 &#61; 5;<br /></code></pre>



<a id="0x1_code_EUPGRADE_IMMUTABLE"></a>

Cannot upgrade an immutable package


<pre><code><b>const</b> <a href="code.md#0x1_code_EUPGRADE_IMMUTABLE">EUPGRADE_IMMUTABLE</a>: u64 &#61; 2;<br /></code></pre>



<a id="0x1_code_EUPGRADE_WEAKER_POLICY"></a>

Cannot downgrade a package&apos;s upgradability policy


<pre><code><b>const</b> <a href="code.md#0x1_code_EUPGRADE_WEAKER_POLICY">EUPGRADE_WEAKER_POLICY</a>: u64 &#61; 3;<br /></code></pre>



<a id="0x1_code_upgrade_policy_arbitrary"></a>

## Function `upgrade_policy_arbitrary`

Whether unconditional code upgrade with no compatibility check is allowed. This
publication mode should only be used for modules which aren&apos;t shared with user others.
The developer is responsible for not breaking memory layout of any resources he already
stored on chain.


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_upgrade_policy_arbitrary">upgrade_policy_arbitrary</a>(): <a href="code.md#0x1_code_UpgradePolicy">code::UpgradePolicy</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_upgrade_policy_arbitrary">upgrade_policy_arbitrary</a>(): <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a> &#123;<br />    <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a> &#123; policy: 0 &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_upgrade_policy_compat"></a>

## Function `upgrade_policy_compat`

Whether a compatibility check should be performed for upgrades. The check only passes if
a new module has (a) the same public functions (b) for existing resources, no layout change.


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_upgrade_policy_compat">upgrade_policy_compat</a>(): <a href="code.md#0x1_code_UpgradePolicy">code::UpgradePolicy</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_upgrade_policy_compat">upgrade_policy_compat</a>(): <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a> &#123;<br />    <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a> &#123; policy: 1 &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_upgrade_policy_immutable"></a>

## Function `upgrade_policy_immutable`

Whether the modules in the package are immutable and cannot be upgraded.


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_upgrade_policy_immutable">upgrade_policy_immutable</a>(): <a href="code.md#0x1_code_UpgradePolicy">code::UpgradePolicy</a><br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_upgrade_policy_immutable">upgrade_policy_immutable</a>(): <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a> &#123;<br />    <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a> &#123; policy: 2 &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_can_change_upgrade_policy_to"></a>

## Function `can_change_upgrade_policy_to`

Whether the upgrade policy can be changed. In general, the policy can be only
strengthened but not weakened.


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_can_change_upgrade_policy_to">can_change_upgrade_policy_to</a>(from: <a href="code.md#0x1_code_UpgradePolicy">code::UpgradePolicy</a>, <b>to</b>: <a href="code.md#0x1_code_UpgradePolicy">code::UpgradePolicy</a>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_can_change_upgrade_policy_to">can_change_upgrade_policy_to</a>(from: <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a>, <b>to</b>: <a href="code.md#0x1_code_UpgradePolicy">UpgradePolicy</a>): bool &#123;<br />    from.policy &lt;&#61; <b>to</b>.policy<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_initialize"></a>

## Function `initialize`

Initialize package metadata for Genesis.


<pre><code><b>fun</b> <a href="code.md#0x1_code_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, package_owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="code.md#0x1_code_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, package_owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>)<br /><b>acquires</b> <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> &#123;<br />    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);<br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(package_owner);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(addr)) &#123;<br />        <b>move_to</b>(package_owner, <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> &#123; packages: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[metadata] &#125;)<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(addr).packages, metadata)<br />    &#125;<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_publish_package"></a>

## Function `publish_package`

Publishes a package at the given signer&apos;s address. The caller must provide package metadata describing the
package.


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_publish_package">publish_package</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pack: <a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_publish_package">publish_package</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pack: <a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) <b>acquires</b> <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> &#123;<br />    // Disallow incompatible upgrade mode. Governance can decide later <b>if</b> this should be reconsidered.<br />    <b>assert</b>!(<br />        pack.upgrade_policy.policy &gt; <a href="code.md#0x1_code_upgrade_policy_arbitrary">upgrade_policy_arbitrary</a>().policy,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="code.md#0x1_code_EINCOMPATIBLE_POLICY_DISABLED">EINCOMPATIBLE_POLICY_DISABLED</a>),<br />    );<br /><br />    <b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br />    <b>if</b> (!<b>exists</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(addr)) &#123;<br />        <b>move_to</b>(owner, <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> &#123; packages: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>() &#125;)<br />    &#125;;<br /><br />    // Checks for valid dependencies <b>to</b> other packages<br />    <b>let</b> allowed_deps &#61; <a href="code.md#0x1_code_check_dependencies">check_dependencies</a>(addr, &amp;pack);<br /><br />    // Check package against conflicts<br />    // To avoid prover compiler <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> on <b>spec</b><br />    // the package need <b>to</b> be an immutable variable<br />    <b>let</b> module_names &#61; <a href="code.md#0x1_code_get_module_names">get_module_names</a>(&amp;pack);<br />    <b>let</b> package_immutable &#61; &amp;<b>borrow_global</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(addr).packages;<br />    <b>let</b> len &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(package_immutable);<br />    <b>let</b> index &#61; len;<br />    <b>let</b> upgrade_number &#61; 0;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_enumerate_ref">vector::enumerate_ref</a>(package_immutable<br />    , &#124;i, <b>old</b>&#124; &#123;<br />        <b>let</b> <b>old</b>: &amp;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a> &#61; <b>old</b>;<br />        <b>if</b> (<b>old</b>.name &#61;&#61; pack.name) &#123;<br />            upgrade_number &#61; <b>old</b>.upgrade_number &#43; 1;<br />            <a href="code.md#0x1_code_check_upgradability">check_upgradability</a>(<b>old</b>, &amp;pack, &amp;module_names);<br />            index &#61; i;<br />        &#125; <b>else</b> &#123;<br />            <a href="code.md#0x1_code_check_coexistence">check_coexistence</a>(<b>old</b>, &amp;module_names)<br />        &#125;;<br />    &#125;);<br /><br />    // Assign the upgrade counter.<br />    pack.upgrade_number &#61; upgrade_number;<br /><br />    <b>let</b> packages &#61; &amp;<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(addr).packages;<br />    // Update registry<br />    <b>let</b> policy &#61; pack.upgrade_policy;<br />    <b>if</b> (index &lt; len) &#123;<br />        &#42;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(packages, index) &#61; pack<br />    &#125; <b>else</b> &#123;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(packages, pack)<br />    &#125;;<br /><br />    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="code.md#0x1_code_PublishPackage">PublishPackage</a> &#123;<br />        code_address: addr,<br />        is_upgrade: upgrade_number &gt; 0<br />    &#125;);<br /><br />    // Request publish<br />    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/features.md#0x1_features_code_dependency_check_enabled">features::code_dependency_check_enabled</a>())<br />        <a href="code.md#0x1_code_request_publish_with_allowed_deps">request_publish_with_allowed_deps</a>(addr, module_names, allowed_deps, <a href="code.md#0x1_code">code</a>, policy.policy)<br />    <b>else</b><br />    // The new `request_publish_with_allowed_deps` <b>has</b> not yet rolled out, so call downwards<br />    // compatible <a href="code.md#0x1_code">code</a>.<br />        <a href="code.md#0x1_code_request_publish">request_publish</a>(addr, module_names, <a href="code.md#0x1_code">code</a>, policy.policy)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_freeze_code_object"></a>

## Function `freeze_code_object`



<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_freeze_code_object">freeze_code_object</a>(publisher: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, code_object: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="code.md#0x1_code_PackageRegistry">code::PackageRegistry</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_freeze_code_object">freeze_code_object</a>(publisher: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, code_object: Object&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;) <b>acquires</b> <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> &#123;<br />    <b>let</b> code_object_addr &#61; <a href="object.md#0x1_object_object_address">object::object_address</a>(&amp;code_object);<br />    <b>assert</b>!(<b>exists</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(code_object_addr), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="code.md#0x1_code_ECODE_OBJECT_DOES_NOT_EXIST">ECODE_OBJECT_DOES_NOT_EXIST</a>));<br />    <b>assert</b>!(<br />        <a href="object.md#0x1_object_is_owner">object::is_owner</a>(code_object, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(publisher)),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="code.md#0x1_code_ENOT_PACKAGE_OWNER">ENOT_PACKAGE_OWNER</a>)<br />    );<br /><br />    <b>let</b> registry &#61; <b>borrow_global_mut</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(code_object_addr);<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_mut">vector::for_each_mut</a>&lt;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>&gt;(&amp;<b>mut</b> registry.packages, &#124;pack&#124; &#123;<br />        <b>let</b> package: &amp;<b>mut</b> <a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a> &#61; pack;<br />        package.upgrade_policy &#61; <a href="code.md#0x1_code_upgrade_policy_immutable">upgrade_policy_immutable</a>();<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_publish_package_txn"></a>

## Function `publish_package_txn`

Same as <code>publish_package</code> but as an entry function which can be called as a transaction. Because
of current restrictions for txn parameters, the metadata needs to be passed in serialized form.


<pre><code><b>public</b> entry <b>fun</b> <a href="code.md#0x1_code_publish_package_txn">publish_package_txn</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="code.md#0x1_code_publish_package_txn">publish_package_txn</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /><b>acquires</b> <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> &#123;<br />    <a href="code.md#0x1_code_publish_package">publish_package</a>(owner, <a href="util.md#0x1_util_from_bytes">util::from_bytes</a>&lt;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>&gt;(metadata_serialized), <a href="code.md#0x1_code">code</a>)<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_check_upgradability"></a>

## Function `check_upgradability`

Checks whether the given package is upgradable, and returns true if a compatibility check is needed.


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_upgradability">check_upgradability</a>(old_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, new_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, new_modules: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_upgradability">check_upgradability</a>(<br />    old_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>, new_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>, new_modules: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;) &#123;<br />    <b>assert</b>!(old_pack.upgrade_policy.policy &lt; <a href="code.md#0x1_code_upgrade_policy_immutable">upgrade_policy_immutable</a>().policy,<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="code.md#0x1_code_EUPGRADE_IMMUTABLE">EUPGRADE_IMMUTABLE</a>));<br />    <b>assert</b>!(<a href="code.md#0x1_code_can_change_upgrade_policy_to">can_change_upgrade_policy_to</a>(old_pack.upgrade_policy, new_pack.upgrade_policy),<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="code.md#0x1_code_EUPGRADE_WEAKER_POLICY">EUPGRADE_WEAKER_POLICY</a>));<br />    <b>let</b> old_modules &#61; <a href="code.md#0x1_code_get_module_names">get_module_names</a>(old_pack);<br /><br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;old_modules, &#124;old_module&#124; &#123;<br />        <b>assert</b>!(<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(new_modules, old_module),<br />            <a href="code.md#0x1_code_EMODULE_MISSING">EMODULE_MISSING</a><br />        );<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_check_coexistence"></a>

## Function `check_coexistence`

Checks whether a new package with given names can co&#45;exist with old package.


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_coexistence">check_coexistence</a>(old_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, new_modules: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_coexistence">check_coexistence</a>(old_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>, new_modules: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;) &#123;<br />    // The modules introduced by each package must not overlap <b>with</b> `names`.<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;old_pack.modules, &#124;old_mod&#124; &#123;<br />        <b>let</b> old_mod: &amp;<a href="code.md#0x1_code_ModuleMetadata">ModuleMetadata</a> &#61; old_mod;<br />        <b>let</b> j &#61; 0;<br />        <b>while</b> (j &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(new_modules)) &#123;<br />            <b>let</b> name &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(new_modules, j);<br />            <b>assert</b>!(&amp;old_mod.name !&#61; name, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_already_exists">error::already_exists</a>(<a href="code.md#0x1_code_EMODULE_NAME_CLASH">EMODULE_NAME_CLASH</a>));<br />            j &#61; j &#43; 1;<br />        &#125;;<br />    &#125;);<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_check_dependencies"></a>

## Function `check_dependencies`

Check that the upgrade policies of all packages are equal or higher quality than this package. Also
compute the list of module dependencies which are allowed by the package metadata. The later
is passed on to the native layer to verify that bytecode dependencies are actually what is pretended here.


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_dependencies">check_dependencies</a>(publish_address: <b>address</b>, pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_AllowedDep">code::AllowedDep</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_dependencies">check_dependencies</a>(publish_address: <b>address</b>, pack: &amp;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_AllowedDep">AllowedDep</a>&gt;<br /><b>acquires</b> <a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a> &#123;<br />    <b>let</b> allowed_module_deps &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <b>let</b> deps &#61; &amp;pack.deps;<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(deps, &#124;dep&#124; &#123;<br />        <b>let</b> dep: &amp;<a href="code.md#0x1_code_PackageDep">PackageDep</a> &#61; dep;<br />        <b>assert</b>!(<b>exists</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(dep.<a href="account.md#0x1_account">account</a>), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="code.md#0x1_code_EPACKAGE_DEP_MISSING">EPACKAGE_DEP_MISSING</a>));<br />        <b>if</b> (<a href="code.md#0x1_code_is_policy_exempted_address">is_policy_exempted_address</a>(dep.<a href="account.md#0x1_account">account</a>)) &#123;<br />            // Allow all modules from this <b>address</b>, by using &quot;&quot; <b>as</b> a wildcard in the <a href="code.md#0x1_code_AllowedDep">AllowedDep</a><br />            <b>let</b> <a href="account.md#0x1_account">account</a>: <b>address</b> &#61; dep.<a href="account.md#0x1_account">account</a>;<br />            <b>let</b> module_name &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b&quot;&quot;);<br />            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> allowed_module_deps, <a href="code.md#0x1_code_AllowedDep">AllowedDep</a> &#123; <a href="account.md#0x1_account">account</a>, module_name &#125;);<br />        &#125; <b>else</b> &#123;<br />            <b>let</b> registry &#61; <b>borrow_global</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(dep.<a href="account.md#0x1_account">account</a>);<br />            <b>let</b> found &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_any">vector::any</a>(&amp;registry.packages, &#124;dep_pack&#124; &#123;<br />                <b>let</b> dep_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a> &#61; dep_pack;<br />                <b>if</b> (dep_pack.name &#61;&#61; dep.package_name) &#123;<br />                    // Check policy<br />                    <b>assert</b>!(<br />                        dep_pack.upgrade_policy.policy &gt;&#61; pack.upgrade_policy.policy,<br />                        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="code.md#0x1_code_EDEP_WEAKER_POLICY">EDEP_WEAKER_POLICY</a>)<br />                    );<br />                    <b>if</b> (dep_pack.upgrade_policy &#61;&#61; <a href="code.md#0x1_code_upgrade_policy_arbitrary">upgrade_policy_arbitrary</a>()) &#123;<br />                        <b>assert</b>!(<br />                            dep.<a href="account.md#0x1_account">account</a> &#61;&#61; publish_address,<br />                            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="code.md#0x1_code_EDEP_ARBITRARY_NOT_SAME_ADDRESS">EDEP_ARBITRARY_NOT_SAME_ADDRESS</a>)<br />                        )<br />                    &#125;;<br />                    // Add allowed deps<br />                    <b>let</b> <a href="account.md#0x1_account">account</a> &#61; dep.<a href="account.md#0x1_account">account</a>;<br />                    <b>let</b> k &#61; 0;<br />                    <b>let</b> r &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&amp;dep_pack.modules);<br />                    <b>while</b> (k &lt; r) &#123;<br />                        <b>let</b> module_name &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&amp;dep_pack.modules, k).name;<br />                        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> allowed_module_deps, <a href="code.md#0x1_code_AllowedDep">AllowedDep</a> &#123; <a href="account.md#0x1_account">account</a>, module_name &#125;);<br />                        k &#61; k &#43; 1;<br />                    &#125;;<br />                    <b>true</b><br />                &#125; <b>else</b> &#123;<br />                    <b>false</b><br />                &#125;<br />            &#125;);<br />            <b>assert</b>!(found, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="code.md#0x1_code_EPACKAGE_DEP_MISSING">EPACKAGE_DEP_MISSING</a>));<br />        &#125;;<br />    &#125;);<br />    allowed_module_deps<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_is_policy_exempted_address"></a>

## Function `is_policy_exempted_address`

Core addresses which are exempted from the check that their policy matches the referring package. Without
this exemption, it would not be possible to define an immutable package based on the core system, which
requires to be upgradable for maintenance and evolution, and is configured to be <code>compatible</code>.


<pre><code><b>fun</b> <a href="code.md#0x1_code_is_policy_exempted_address">is_policy_exempted_address</a>(addr: <b>address</b>): bool<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="code.md#0x1_code_is_policy_exempted_address">is_policy_exempted_address</a>(addr: <b>address</b>): bool &#123;<br />    addr &#61;&#61; @1 &#124;&#124; addr &#61;&#61; @2 &#124;&#124; addr &#61;&#61; @3 &#124;&#124; addr &#61;&#61; @4 &#124;&#124; addr &#61;&#61; @5 &#124;&#124;<br />        addr &#61;&#61; @6 &#124;&#124; addr &#61;&#61; @7 &#124;&#124; addr &#61;&#61; @8 &#124;&#124; addr &#61;&#61; @9 &#124;&#124; addr &#61;&#61; @10<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_get_module_names"></a>

## Function `get_module_names`

Get the names of the modules in a package.


<pre><code><b>fun</b> <a href="code.md#0x1_code_get_module_names">get_module_names</a>(pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="code.md#0x1_code_get_module_names">get_module_names</a>(pack: &amp;<a href="code.md#0x1_code_PackageMetadata">PackageMetadata</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt; &#123;<br />    <b>let</b> module_names &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>();<br />    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&amp;pack.modules, &#124;pack_module&#124; &#123;<br />        <b>let</b> pack_module: &amp;<a href="code.md#0x1_code_ModuleMetadata">ModuleMetadata</a> &#61; pack_module;<br />        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&amp;<b>mut</b> module_names, pack_module.name);<br />    &#125;);<br />    module_names<br />&#125;<br /></code></pre>



</details>

<a id="0x1_code_request_publish"></a>

## Function `request_publish`

Native function to initiate module loading


<pre><code><b>fun</b> <a href="code.md#0x1_code_request_publish">request_publish</a>(owner: <b>address</b>, expected_modules: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, bundle: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, policy: u8)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="code.md#0x1_code_request_publish">request_publish</a>(<br />    owner: <b>address</b>,<br />    expected_modules: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    bundle: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    policy: u8<br />);<br /></code></pre>



</details>

<a id="0x1_code_request_publish_with_allowed_deps"></a>

## Function `request_publish_with_allowed_deps`

Native function to initiate module loading, including a list of allowed dependencies.


<pre><code><b>fun</b> <a href="code.md#0x1_code_request_publish_with_allowed_deps">request_publish_with_allowed_deps</a>(owner: <b>address</b>, expected_modules: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, allowed_deps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_AllowedDep">code::AllowedDep</a>&gt;, bundle: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, policy: u8)<br /></code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="code.md#0x1_code_request_publish_with_allowed_deps">request_publish_with_allowed_deps</a>(<br />    owner: <b>address</b>,<br />    expected_modules: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;String&gt;,<br />    allowed_deps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_AllowedDep">AllowedDep</a>&gt;,<br />    bundle: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,<br />    policy: u8<br />);<br /></code></pre>



</details>

<a id="@Specification_1"></a>

## Specification




<a id="high-level-req"></a>

### High-level Requirements

<table>
<tr>
<th>No.</th><th>Requirement</th><th>Criticality</th><th>Implementation</th><th>Enforcement</th>
</tr>

<tr>
<td>1</td>
<td>Updating a package should fail if the user is not the owner of it.</td>
<td>Critical</td>
<td>The publish_package function may only be able to update the package if the signer is the actual owner of the package.</td>
<td>The Aptos upgrade native functions have been manually audited.</td>
</tr>

<tr>
<td>2</td>
<td>The arbitrary upgrade policy should never be used.</td>
<td>Critical</td>
<td>There should never be a pass of an arbitrary upgrade policy to the request_publish native function.</td>
<td>Manually audited that it aborts if package.upgrade_policy.policy &#61;&#61; 0.</td>
</tr>

<tr>
<td>3</td>
<td>Should perform accurate compatibility checks when the policy indicates compatibility, ensuring it meets the required conditions.</td>
<td>Critical</td>
<td>Specifies if it should perform compatibility checks for upgrades. The check only passes if a new module has (a) the same public functions, and (b) for existing resources, no layout change.</td>
<td>The Move upgradability patterns have been manually audited.</td>
</tr>

<tr>
<td>4</td>
<td>Package upgrades should abide by policy change rules. In particular, The new upgrade policy must be equal to or stricter when compared to the old one. The original upgrade policy must not be immutable. The new package must contain all modules contained in the old package.</td>
<td>Medium</td>
<td>A package may only be updated using the publish_package function when the check_upgradability function returns true.</td>
<td>This is audited by a manual review of the check_upgradability patterns.</td>
</tr>

<tr>
<td>5</td>
<td>The upgrade policy of a package must not exceed the strictness level imposed by its dependencies.</td>
<td>Medium</td>
<td>The upgrade_policy of a package may only be less than its dependencies throughout the upgrades. In addition, the native code properly restricts the use of dependencies outside the passed&#45;in metadata.</td>
<td>This has been manually audited.</td>
</tr>

<tr>
<td>6</td>
<td>The extension for package metadata is currently unused.</td>
<td>Medium</td>
<td>The extension field in PackageMetadata should be unused.</td>
<td>Data invariant on the extension field has been manually audited.</td>
</tr>

<tr>
<td>7</td>
<td>The upgrade number of a package increases incrementally in a monotonic manner with each subsequent upgrade.</td>
<td>Low</td>
<td>On each upgrade of a particular package, the publish_package function updates the upgrade_number for that package.</td>
<td>Post condition on upgrade_number has been manually audited.</td>
</tr>

</table>




<a id="module-level-spec"></a>

### Module-level Specification


<pre><code><b>pragma</b> verify &#61; <b>true</b>;<br /><b>pragma</b> aborts_if_is_strict;<br /></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>fun</b> <a href="code.md#0x1_code_initialize">initialize</a>(aptos_framework: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, package_owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata: <a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>)<br /></code></pre>




<pre><code><b>let</b> aptos_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(aptos_framework);<br /><b>let</b> owner_addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(package_owner);<br /><b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(aptos_addr);<br /><b>ensures</b> <b>exists</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(owner_addr);<br /></code></pre>



<a id="@Specification_1_publish_package"></a>

### Function `publish_package`


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_publish_package">publish_package</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, pack: <a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> addr &#61; <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner);<br /><b>modifies</b> <b>global</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(addr);<br /><b>aborts_if</b> pack.upgrade_policy.policy &lt;&#61; <a href="code.md#0x1_code_upgrade_policy_arbitrary">upgrade_policy_arbitrary</a>().policy;<br /></code></pre>



<a id="@Specification_1_freeze_code_object"></a>

### Function `freeze_code_object`


<pre><code><b>public</b> <b>fun</b> <a href="code.md#0x1_code_freeze_code_object">freeze_code_object</a>(publisher: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, code_object: <a href="object.md#0x1_object_Object">object::Object</a>&lt;<a href="code.md#0x1_code_PackageRegistry">code::PackageRegistry</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>let</b> code_object_addr &#61; code_object.inner;<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="object.md#0x1_object_ObjectCore">object::ObjectCore</a>&gt;(code_object_addr);<br /><b>aborts_if</b> !<b>exists</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(code_object_addr);<br /><b>aborts_if</b> !<a href="object.md#0x1_object_is_owner">object::is_owner</a>(code_object, <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(publisher));<br /><b>modifies</b> <b>global</b>&lt;<a href="code.md#0x1_code_PackageRegistry">PackageRegistry</a>&gt;(code_object_addr);<br /></code></pre>



<a id="@Specification_1_publish_package_txn"></a>

### Function `publish_package_txn`


<pre><code><b>public</b> entry <b>fun</b> <a href="code.md#0x1_code_publish_package_txn">publish_package_txn</a>(owner: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, metadata_serialized: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="code.md#0x1_code">code</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_check_upgradability"></a>

### Function `check_upgradability`


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_upgradability">check_upgradability</a>(old_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, new_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, new_modules: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> aborts_if_is_partial;<br /><b>aborts_if</b> old_pack.upgrade_policy.policy &gt;&#61; <a href="code.md#0x1_code_upgrade_policy_immutable">upgrade_policy_immutable</a>().policy;<br /><b>aborts_if</b> !<a href="code.md#0x1_code_can_change_upgrade_policy_to">can_change_upgrade_policy_to</a>(old_pack.upgrade_policy, new_pack.upgrade_policy);<br /></code></pre>



<a id="@Specification_1_check_coexistence"></a>

### Function `check_coexistence`


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_coexistence">check_coexistence</a>(old_pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>, new_modules: &amp;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;)<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_check_dependencies"></a>

### Function `check_dependencies`


<pre><code><b>fun</b> <a href="code.md#0x1_code_check_dependencies">check_dependencies</a>(publish_address: <b>address</b>, pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_AllowedDep">code::AllowedDep</a>&gt;<br /></code></pre>




<pre><code><b>pragma</b> verify &#61; <b>false</b>;<br /></code></pre>



<a id="@Specification_1_get_module_names"></a>

### Function `get_module_names`


<pre><code><b>fun</b> <a href="code.md#0x1_code_get_module_names">get_module_names</a>(pack: &amp;<a href="code.md#0x1_code_PackageMetadata">code::PackageMetadata</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /><b>aborts_if</b> [abstract] <b>false</b>;<br /><b>ensures</b> [abstract] len(result) &#61;&#61; len(pack.modules);<br /><b>ensures</b> [abstract] <b>forall</b> i in 0..len(result): result[i] &#61;&#61; pack.modules[i].name;<br /></code></pre>



<a id="@Specification_1_request_publish"></a>

### Function `request_publish`


<pre><code><b>fun</b> <a href="code.md#0x1_code_request_publish">request_publish</a>(owner: <b>address</b>, expected_modules: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, bundle: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, policy: u8)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>



<a id="@Specification_1_request_publish_with_allowed_deps"></a>

### Function `request_publish_with_allowed_deps`


<pre><code><b>fun</b> <a href="code.md#0x1_code_request_publish_with_allowed_deps">request_publish_with_allowed_deps</a>(owner: <b>address</b>, expected_modules: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>&gt;, allowed_deps: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="code.md#0x1_code_AllowedDep">code::AllowedDep</a>&gt;, bundle: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, policy: u8)<br /></code></pre>




<pre><code><b>pragma</b> opaque;<br /></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
