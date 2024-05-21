
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


<pre><code>use 0x1::copyable_any;<br/>use 0x1::error;<br/>use 0x1::event;<br/>use 0x1::features;<br/>use 0x1::object;<br/>use 0x1::option;<br/>use 0x1::signer;<br/>use 0x1::string;<br/>use 0x1::system_addresses;<br/>use 0x1::util;<br/>use 0x1::vector;<br/></code></pre>



<a id="0x1_code_PackageRegistry"></a>

## Resource `PackageRegistry`

The package registry at the given address.


<pre><code>struct PackageRegistry has drop, store, key<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>packages: vector&lt;code::PackageMetadata&gt;</code>
</dt>
<dd>
 Packages installed at this address.
</dd>
</dl>


</details>

<a id="0x1_code_PackageMetadata"></a>

## Struct `PackageMetadata`

Metadata for a package. All byte blobs are represented as base64-of-gzipped-bytes


<pre><code>struct PackageMetadata has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 Name of this package.
</dd>
<dt>
<code>upgrade_policy: code::UpgradePolicy</code>
</dt>
<dd>
 The upgrade policy of this package.
</dd>
<dt>
<code>upgrade_number: u64</code>
</dt>
<dd>
 The numbers of times this module has been upgraded. Also serves as the on-chain version.
 This field will be automatically assigned on successful upgrade.
</dd>
<dt>
<code>source_digest: string::String</code>
</dt>
<dd>
 The source digest of the sources in the package. This is constructed by first building the
 sha256 of each individual source, than sorting them alphabetically, and sha256 them again.
</dd>
<dt>
<code>manifest: vector&lt;u8&gt;</code>
</dt>
<dd>
 The package manifest, in the Move.toml format. Gzipped text.
</dd>
<dt>
<code>modules: vector&lt;code::ModuleMetadata&gt;</code>
</dt>
<dd>
 The list of modules installed by this package.
</dd>
<dt>
<code>deps: vector&lt;code::PackageDep&gt;</code>
</dt>
<dd>
 Holds PackageDeps.
</dd>
<dt>
<code>extension: option::Option&lt;copyable_any::Any&gt;</code>
</dt>
<dd>
 For future extension
</dd>
</dl>


</details>

<a id="0x1_code_PackageDep"></a>

## Struct `PackageDep`

A dependency to a package published at address


<pre><code>struct PackageDep has copy, drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
</dt>
<dd>

</dd>
<dt>
<code>package_name: string::String</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_code_ModuleMetadata"></a>

## Struct `ModuleMetadata`

Metadata about a module in a package.


<pre><code>struct ModuleMetadata has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>name: string::String</code>
</dt>
<dd>
 Name of the module.
</dd>
<dt>
<code>source: vector&lt;u8&gt;</code>
</dt>
<dd>
 Source text, gzipped String. Empty if not provided.
</dd>
<dt>
<code>source_map: vector&lt;u8&gt;</code>
</dt>
<dd>
 Source map, in compressed BCS. Empty if not provided.
</dd>
<dt>
<code>extension: option::Option&lt;copyable_any::Any&gt;</code>
</dt>
<dd>
 For future extensions.
</dd>
</dl>


</details>

<a id="0x1_code_UpgradePolicy"></a>

## Struct `UpgradePolicy`

Describes an upgrade policy


<pre><code>struct UpgradePolicy has copy, drop, store<br/></code></pre>



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


<pre><code>&#35;[event]<br/>struct PublishPackage has drop, store<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>code_address: address</code>
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


<pre><code>struct AllowedDep has drop<br/></code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>account: address</code>
</dt>
<dd>
 Address of the module.
</dd>
<dt>
<code>module_name: string::String</code>
</dt>
<dd>
 Name of the module. If this is the empty string, then this serves as a wildcard for
 all modules from this address. This is used for speeding up dependency checking for packages from
 well-known framework addresses, where we can assume that there are no malicious packages.
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_code_ECODE_OBJECT_DOES_NOT_EXIST"></a>

<code>code_object</code> does not exist.


<pre><code>const ECODE_OBJECT_DOES_NOT_EXIST: u64 &#61; 10;<br/></code></pre>



<a id="0x1_code_EDEP_ARBITRARY_NOT_SAME_ADDRESS"></a>

A dependency to an <code>arbitrary</code> package must be on the same address.


<pre><code>const EDEP_ARBITRARY_NOT_SAME_ADDRESS: u64 &#61; 7;<br/></code></pre>



<a id="0x1_code_EDEP_WEAKER_POLICY"></a>

A dependency cannot have a weaker upgrade policy.


<pre><code>const EDEP_WEAKER_POLICY: u64 &#61; 6;<br/></code></pre>



<a id="0x1_code_EINCOMPATIBLE_POLICY_DISABLED"></a>

Creating a package with incompatible upgrade policy is disabled.


<pre><code>const EINCOMPATIBLE_POLICY_DISABLED: u64 &#61; 8;<br/></code></pre>



<a id="0x1_code_EMODULE_MISSING"></a>

Cannot delete a module that was published in the same package


<pre><code>const EMODULE_MISSING: u64 &#61; 4;<br/></code></pre>



<a id="0x1_code_EMODULE_NAME_CLASH"></a>

Package contains duplicate module names with existing modules publised in other packages on this address


<pre><code>const EMODULE_NAME_CLASH: u64 &#61; 1;<br/></code></pre>



<a id="0x1_code_ENOT_PACKAGE_OWNER"></a>

Not the owner of the package registry.


<pre><code>const ENOT_PACKAGE_OWNER: u64 &#61; 9;<br/></code></pre>



<a id="0x1_code_EPACKAGE_DEP_MISSING"></a>

Dependency could not be resolved to any published package.


<pre><code>const EPACKAGE_DEP_MISSING: u64 &#61; 5;<br/></code></pre>



<a id="0x1_code_EUPGRADE_IMMUTABLE"></a>

Cannot upgrade an immutable package


<pre><code>const EUPGRADE_IMMUTABLE: u64 &#61; 2;<br/></code></pre>



<a id="0x1_code_EUPGRADE_WEAKER_POLICY"></a>

Cannot downgrade a package's upgradability policy


<pre><code>const EUPGRADE_WEAKER_POLICY: u64 &#61; 3;<br/></code></pre>



<a id="0x1_code_upgrade_policy_arbitrary"></a>

## Function `upgrade_policy_arbitrary`

Whether unconditional code upgrade with no compatibility check is allowed. This
publication mode should only be used for modules which aren't shared with user others.
The developer is responsible for not breaking memory layout of any resources he already
stored on chain.


<pre><code>public fun upgrade_policy_arbitrary(): code::UpgradePolicy<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_policy_arbitrary(): UpgradePolicy &#123;<br/>    UpgradePolicy &#123; policy: 0 &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_upgrade_policy_compat"></a>

## Function `upgrade_policy_compat`

Whether a compatibility check should be performed for upgrades. The check only passes if
a new module has (a) the same public functions (b) for existing resources, no layout change.


<pre><code>public fun upgrade_policy_compat(): code::UpgradePolicy<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_policy_compat(): UpgradePolicy &#123;<br/>    UpgradePolicy &#123; policy: 1 &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_upgrade_policy_immutable"></a>

## Function `upgrade_policy_immutable`

Whether the modules in the package are immutable and cannot be upgraded.


<pre><code>public fun upgrade_policy_immutable(): code::UpgradePolicy<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun upgrade_policy_immutable(): UpgradePolicy &#123;<br/>    UpgradePolicy &#123; policy: 2 &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_can_change_upgrade_policy_to"></a>

## Function `can_change_upgrade_policy_to`

Whether the upgrade policy can be changed. In general, the policy can be only
strengthened but not weakened.


<pre><code>public fun can_change_upgrade_policy_to(from: code::UpgradePolicy, to: code::UpgradePolicy): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun can_change_upgrade_policy_to(from: UpgradePolicy, to: UpgradePolicy): bool &#123;<br/>    from.policy &lt;&#61; to.policy<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_initialize"></a>

## Function `initialize`

Initialize package metadata for Genesis.


<pre><code>fun initialize(aptos_framework: &amp;signer, package_owner: &amp;signer, metadata: code::PackageMetadata)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun initialize(aptos_framework: &amp;signer, package_owner: &amp;signer, metadata: PackageMetadata)<br/>acquires PackageRegistry &#123;<br/>    system_addresses::assert_aptos_framework(aptos_framework);<br/>    let addr &#61; signer::address_of(package_owner);<br/>    if (!exists&lt;PackageRegistry&gt;(addr)) &#123;<br/>        move_to(package_owner, PackageRegistry &#123; packages: vector[metadata] &#125;)<br/>    &#125; else &#123;<br/>        vector::push_back(&amp;mut borrow_global_mut&lt;PackageRegistry&gt;(addr).packages, metadata)<br/>    &#125;<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_publish_package"></a>

## Function `publish_package`

Publishes a package at the given signer's address. The caller must provide package metadata describing the
package.


<pre><code>public fun publish_package(owner: &amp;signer, pack: code::PackageMetadata, code: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun publish_package(owner: &amp;signer, pack: PackageMetadata, code: vector&lt;vector&lt;u8&gt;&gt;) acquires PackageRegistry &#123;<br/>    // Disallow incompatible upgrade mode. Governance can decide later if this should be reconsidered.<br/>    assert!(<br/>        pack.upgrade_policy.policy &gt; upgrade_policy_arbitrary().policy,<br/>        error::invalid_argument(EINCOMPATIBLE_POLICY_DISABLED),<br/>    );<br/><br/>    let addr &#61; signer::address_of(owner);<br/>    if (!exists&lt;PackageRegistry&gt;(addr)) &#123;<br/>        move_to(owner, PackageRegistry &#123; packages: vector::empty() &#125;)<br/>    &#125;;<br/><br/>    // Checks for valid dependencies to other packages<br/>    let allowed_deps &#61; check_dependencies(addr, &amp;pack);<br/><br/>    // Check package against conflicts<br/>    // To avoid prover compiler error on spec<br/>    // the package need to be an immutable variable<br/>    let module_names &#61; get_module_names(&amp;pack);<br/>    let package_immutable &#61; &amp;borrow_global&lt;PackageRegistry&gt;(addr).packages;<br/>    let len &#61; vector::length(package_immutable);<br/>    let index &#61; len;<br/>    let upgrade_number &#61; 0;<br/>    vector::enumerate_ref(package_immutable<br/>    , &#124;i, old&#124; &#123;<br/>        let old: &amp;PackageMetadata &#61; old;<br/>        if (old.name &#61;&#61; pack.name) &#123;<br/>            upgrade_number &#61; old.upgrade_number &#43; 1;<br/>            check_upgradability(old, &amp;pack, &amp;module_names);<br/>            index &#61; i;<br/>        &#125; else &#123;<br/>            check_coexistence(old, &amp;module_names)<br/>        &#125;;<br/>    &#125;);<br/><br/>    // Assign the upgrade counter.<br/>    pack.upgrade_number &#61; upgrade_number;<br/><br/>    let packages &#61; &amp;mut borrow_global_mut&lt;PackageRegistry&gt;(addr).packages;<br/>    // Update registry<br/>    let policy &#61; pack.upgrade_policy;<br/>    if (index &lt; len) &#123;<br/>        &#42;vector::borrow_mut(packages, index) &#61; pack<br/>    &#125; else &#123;<br/>        vector::push_back(packages, pack)<br/>    &#125;;<br/><br/>    event::emit(PublishPackage &#123;<br/>        code_address: addr,<br/>        is_upgrade: upgrade_number &gt; 0<br/>    &#125;);<br/><br/>    // Request publish<br/>    if (features::code_dependency_check_enabled())<br/>        request_publish_with_allowed_deps(addr, module_names, allowed_deps, code, policy.policy)<br/>    else<br/>    // The new `request_publish_with_allowed_deps` has not yet rolled out, so call downwards<br/>    // compatible code.<br/>        request_publish(addr, module_names, code, policy.policy)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_freeze_code_object"></a>

## Function `freeze_code_object`



<pre><code>public fun freeze_code_object(publisher: &amp;signer, code_object: object::Object&lt;code::PackageRegistry&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public fun freeze_code_object(publisher: &amp;signer, code_object: Object&lt;PackageRegistry&gt;) acquires PackageRegistry &#123;<br/>    let code_object_addr &#61; object::object_address(&amp;code_object);<br/>    assert!(exists&lt;PackageRegistry&gt;(code_object_addr), error::not_found(ECODE_OBJECT_DOES_NOT_EXIST));<br/>    assert!(<br/>        object::is_owner(code_object, signer::address_of(publisher)),<br/>        error::permission_denied(ENOT_PACKAGE_OWNER)<br/>    );<br/><br/>    let registry &#61; borrow_global_mut&lt;PackageRegistry&gt;(code_object_addr);<br/>    vector::for_each_mut&lt;PackageMetadata&gt;(&amp;mut registry.packages, &#124;pack&#124; &#123;<br/>        let package: &amp;mut PackageMetadata &#61; pack;<br/>        package.upgrade_policy &#61; upgrade_policy_immutable();<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_publish_package_txn"></a>

## Function `publish_package_txn`

Same as <code>publish_package</code> but as an entry function which can be called as a transaction. Because
of current restrictions for txn parameters, the metadata needs to be passed in serialized form.


<pre><code>public entry fun publish_package_txn(owner: &amp;signer, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>public entry fun publish_package_txn(owner: &amp;signer, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)<br/>acquires PackageRegistry &#123;<br/>    publish_package(owner, util::from_bytes&lt;PackageMetadata&gt;(metadata_serialized), code)<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_check_upgradability"></a>

## Function `check_upgradability`

Checks whether the given package is upgradable, and returns true if a compatibility check is needed.


<pre><code>fun check_upgradability(old_pack: &amp;code::PackageMetadata, new_pack: &amp;code::PackageMetadata, new_modules: &amp;vector&lt;string::String&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun check_upgradability(<br/>    old_pack: &amp;PackageMetadata, new_pack: &amp;PackageMetadata, new_modules: &amp;vector&lt;String&gt;) &#123;<br/>    assert!(old_pack.upgrade_policy.policy &lt; upgrade_policy_immutable().policy,<br/>        error::invalid_argument(EUPGRADE_IMMUTABLE));<br/>    assert!(can_change_upgrade_policy_to(old_pack.upgrade_policy, new_pack.upgrade_policy),<br/>        error::invalid_argument(EUPGRADE_WEAKER_POLICY));<br/>    let old_modules &#61; get_module_names(old_pack);<br/><br/>    vector::for_each_ref(&amp;old_modules, &#124;old_module&#124; &#123;<br/>        assert!(<br/>            vector::contains(new_modules, old_module),<br/>            EMODULE_MISSING<br/>        );<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_check_coexistence"></a>

## Function `check_coexistence`

Checks whether a new package with given names can co-exist with old package.


<pre><code>fun check_coexistence(old_pack: &amp;code::PackageMetadata, new_modules: &amp;vector&lt;string::String&gt;)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun check_coexistence(old_pack: &amp;PackageMetadata, new_modules: &amp;vector&lt;String&gt;) &#123;<br/>    // The modules introduced by each package must not overlap with `names`.<br/>    vector::for_each_ref(&amp;old_pack.modules, &#124;old_mod&#124; &#123;<br/>        let old_mod: &amp;ModuleMetadata &#61; old_mod;<br/>        let j &#61; 0;<br/>        while (j &lt; vector::length(new_modules)) &#123;<br/>            let name &#61; vector::borrow(new_modules, j);<br/>            assert!(&amp;old_mod.name !&#61; name, error::already_exists(EMODULE_NAME_CLASH));<br/>            j &#61; j &#43; 1;<br/>        &#125;;<br/>    &#125;);<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_check_dependencies"></a>

## Function `check_dependencies`

Check that the upgrade policies of all packages are equal or higher quality than this package. Also
compute the list of module dependencies which are allowed by the package metadata. The later
is passed on to the native layer to verify that bytecode dependencies are actually what is pretended here.


<pre><code>fun check_dependencies(publish_address: address, pack: &amp;code::PackageMetadata): vector&lt;code::AllowedDep&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun check_dependencies(publish_address: address, pack: &amp;PackageMetadata): vector&lt;AllowedDep&gt;<br/>acquires PackageRegistry &#123;<br/>    let allowed_module_deps &#61; vector::empty();<br/>    let deps &#61; &amp;pack.deps;<br/>    vector::for_each_ref(deps, &#124;dep&#124; &#123;<br/>        let dep: &amp;PackageDep &#61; dep;<br/>        assert!(exists&lt;PackageRegistry&gt;(dep.account), error::not_found(EPACKAGE_DEP_MISSING));<br/>        if (is_policy_exempted_address(dep.account)) &#123;<br/>            // Allow all modules from this address, by using &quot;&quot; as a wildcard in the AllowedDep<br/>            let account: address &#61; dep.account;<br/>            let module_name &#61; string::utf8(b&quot;&quot;);<br/>            vector::push_back(&amp;mut allowed_module_deps, AllowedDep &#123; account, module_name &#125;);<br/>        &#125; else &#123;<br/>            let registry &#61; borrow_global&lt;PackageRegistry&gt;(dep.account);<br/>            let found &#61; vector::any(&amp;registry.packages, &#124;dep_pack&#124; &#123;<br/>                let dep_pack: &amp;PackageMetadata &#61; dep_pack;<br/>                if (dep_pack.name &#61;&#61; dep.package_name) &#123;<br/>                    // Check policy<br/>                    assert!(<br/>                        dep_pack.upgrade_policy.policy &gt;&#61; pack.upgrade_policy.policy,<br/>                        error::invalid_argument(EDEP_WEAKER_POLICY)<br/>                    );<br/>                    if (dep_pack.upgrade_policy &#61;&#61; upgrade_policy_arbitrary()) &#123;<br/>                        assert!(<br/>                            dep.account &#61;&#61; publish_address,<br/>                            error::invalid_argument(EDEP_ARBITRARY_NOT_SAME_ADDRESS)<br/>                        )<br/>                    &#125;;<br/>                    // Add allowed deps<br/>                    let account &#61; dep.account;<br/>                    let k &#61; 0;<br/>                    let r &#61; vector::length(&amp;dep_pack.modules);<br/>                    while (k &lt; r) &#123;<br/>                        let module_name &#61; vector::borrow(&amp;dep_pack.modules, k).name;<br/>                        vector::push_back(&amp;mut allowed_module_deps, AllowedDep &#123; account, module_name &#125;);<br/>                        k &#61; k &#43; 1;<br/>                    &#125;;<br/>                    true<br/>                &#125; else &#123;<br/>                    false<br/>                &#125;<br/>            &#125;);<br/>            assert!(found, error::not_found(EPACKAGE_DEP_MISSING));<br/>        &#125;;<br/>    &#125;);<br/>    allowed_module_deps<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_is_policy_exempted_address"></a>

## Function `is_policy_exempted_address`

Core addresses which are exempted from the check that their policy matches the referring package. Without
this exemption, it would not be possible to define an immutable package based on the core system, which
requires to be upgradable for maintenance and evolution, and is configured to be <code>compatible</code>.


<pre><code>fun is_policy_exempted_address(addr: address): bool<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun is_policy_exempted_address(addr: address): bool &#123;<br/>    addr &#61;&#61; @1 &#124;&#124; addr &#61;&#61; @2 &#124;&#124; addr &#61;&#61; @3 &#124;&#124; addr &#61;&#61; @4 &#124;&#124; addr &#61;&#61; @5 &#124;&#124;<br/>        addr &#61;&#61; @6 &#124;&#124; addr &#61;&#61; @7 &#124;&#124; addr &#61;&#61; @8 &#124;&#124; addr &#61;&#61; @9 &#124;&#124; addr &#61;&#61; @10<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_get_module_names"></a>

## Function `get_module_names`

Get the names of the modules in a package.


<pre><code>fun get_module_names(pack: &amp;code::PackageMetadata): vector&lt;string::String&gt;<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>fun get_module_names(pack: &amp;PackageMetadata): vector&lt;String&gt; &#123;<br/>    let module_names &#61; vector::empty();<br/>    vector::for_each_ref(&amp;pack.modules, &#124;pack_module&#124; &#123;<br/>        let pack_module: &amp;ModuleMetadata &#61; pack_module;<br/>        vector::push_back(&amp;mut module_names, pack_module.name);<br/>    &#125;);<br/>    module_names<br/>&#125;<br/></code></pre>



</details>

<a id="0x1_code_request_publish"></a>

## Function `request_publish`

Native function to initiate module loading


<pre><code>fun request_publish(owner: address, expected_modules: vector&lt;string::String&gt;, bundle: vector&lt;vector&lt;u8&gt;&gt;, policy: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun request_publish(<br/>    owner: address,<br/>    expected_modules: vector&lt;String&gt;,<br/>    bundle: vector&lt;vector&lt;u8&gt;&gt;,<br/>    policy: u8<br/>);<br/></code></pre>



</details>

<a id="0x1_code_request_publish_with_allowed_deps"></a>

## Function `request_publish_with_allowed_deps`

Native function to initiate module loading, including a list of allowed dependencies.


<pre><code>fun request_publish_with_allowed_deps(owner: address, expected_modules: vector&lt;string::String&gt;, allowed_deps: vector&lt;code::AllowedDep&gt;, bundle: vector&lt;vector&lt;u8&gt;&gt;, policy: u8)<br/></code></pre>



<details>
<summary>Implementation</summary>


<pre><code>native fun request_publish_with_allowed_deps(<br/>    owner: address,<br/>    expected_modules: vector&lt;String&gt;,<br/>    allowed_deps: vector&lt;AllowedDep&gt;,<br/>    bundle: vector&lt;vector&lt;u8&gt;&gt;,<br/>    policy: u8<br/>);<br/></code></pre>



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
<td>Manually audited that it aborts if package.upgrade_policy.policy == 0.</td>
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
<td>The upgrade_policy of a package may only be less than its dependencies throughout the upgrades. In addition, the native code properly restricts the use of dependencies outside the passed-in metadata.</td>
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


<pre><code>pragma verify &#61; true;<br/>pragma aborts_if_is_strict;<br/></code></pre>



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code>fun initialize(aptos_framework: &amp;signer, package_owner: &amp;signer, metadata: code::PackageMetadata)<br/></code></pre>




<pre><code>let aptos_addr &#61; signer::address_of(aptos_framework);<br/>let owner_addr &#61; signer::address_of(package_owner);<br/>aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);<br/>ensures exists&lt;PackageRegistry&gt;(owner_addr);<br/></code></pre>



<a id="@Specification_1_publish_package"></a>

### Function `publish_package`


<pre><code>public fun publish_package(owner: &amp;signer, pack: code::PackageMetadata, code: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let addr &#61; signer::address_of(owner);<br/>modifies global&lt;PackageRegistry&gt;(addr);<br/>aborts_if pack.upgrade_policy.policy &lt;&#61; upgrade_policy_arbitrary().policy;<br/></code></pre>



<a id="@Specification_1_freeze_code_object"></a>

### Function `freeze_code_object`


<pre><code>public fun freeze_code_object(publisher: &amp;signer, code_object: object::Object&lt;code::PackageRegistry&gt;)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>let code_object_addr &#61; code_object.inner;<br/>aborts_if !exists&lt;object::ObjectCore&gt;(code_object_addr);<br/>aborts_if !exists&lt;PackageRegistry&gt;(code_object_addr);<br/>aborts_if !object::is_owner(code_object, signer::address_of(publisher));<br/>modifies global&lt;PackageRegistry&gt;(code_object_addr);<br/></code></pre>



<a id="@Specification_1_publish_package_txn"></a>

### Function `publish_package_txn`


<pre><code>public entry fun publish_package_txn(owner: &amp;signer, metadata_serialized: vector&lt;u8&gt;, code: vector&lt;vector&lt;u8&gt;&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_check_upgradability"></a>

### Function `check_upgradability`


<pre><code>fun check_upgradability(old_pack: &amp;code::PackageMetadata, new_pack: &amp;code::PackageMetadata, new_modules: &amp;vector&lt;string::String&gt;)<br/></code></pre>




<pre><code>pragma aborts_if_is_partial;<br/>aborts_if old_pack.upgrade_policy.policy &gt;&#61; upgrade_policy_immutable().policy;<br/>aborts_if !can_change_upgrade_policy_to(old_pack.upgrade_policy, new_pack.upgrade_policy);<br/></code></pre>



<a id="@Specification_1_check_coexistence"></a>

### Function `check_coexistence`


<pre><code>fun check_coexistence(old_pack: &amp;code::PackageMetadata, new_modules: &amp;vector&lt;string::String&gt;)<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_check_dependencies"></a>

### Function `check_dependencies`


<pre><code>fun check_dependencies(publish_address: address, pack: &amp;code::PackageMetadata): vector&lt;code::AllowedDep&gt;<br/></code></pre>




<pre><code>pragma verify &#61; false;<br/></code></pre>



<a id="@Specification_1_get_module_names"></a>

### Function `get_module_names`


<pre><code>fun get_module_names(pack: &amp;code::PackageMetadata): vector&lt;string::String&gt;<br/></code></pre>




<pre><code>pragma opaque;<br/>aborts_if [abstract] false;<br/>ensures [abstract] len(result) &#61;&#61; len(pack.modules);<br/>ensures [abstract] forall i in 0..len(result): result[i] &#61;&#61; pack.modules[i].name;<br/></code></pre>



<a id="@Specification_1_request_publish"></a>

### Function `request_publish`


<pre><code>fun request_publish(owner: address, expected_modules: vector&lt;string::String&gt;, bundle: vector&lt;vector&lt;u8&gt;&gt;, policy: u8)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>



<a id="@Specification_1_request_publish_with_allowed_deps"></a>

### Function `request_publish_with_allowed_deps`


<pre><code>fun request_publish_with_allowed_deps(owner: address, expected_modules: vector&lt;string::String&gt;, allowed_deps: vector&lt;code::AllowedDep&gt;, bundle: vector&lt;vector&lt;u8&gt;&gt;, policy: u8)<br/></code></pre>




<pre><code>pragma opaque;<br/></code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
