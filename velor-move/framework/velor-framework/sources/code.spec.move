spec velor_framework::code {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Updating a package should fail if the user is not the owner of it.
    /// Criticality: Critical
    /// Implementation: The publish_package function may only be able to update the package if the signer is the actual
    /// owner of the package.
    /// Enforcement: The Velor upgrade native functions have been manually audited.
    ///
    /// No.: 2
    /// Requirement: The arbitrary upgrade policy should never be used.
    /// Criticality: Critical
    /// Implementation: There should never be a pass of an arbitrary upgrade policy to the
    /// request_publish native function.
    /// Enforcement: Manually audited that it aborts if package.upgrade_policy.policy == 0.
    ///
    /// No.: 3
    /// Requirement: Should perform accurate compatibility checks when the policy indicates
    /// compatibility, ensuring it meets the required conditions.
    /// Criticality: Critical
    /// Implementation: Specifies if it should perform compatibility checks for upgrades. The check
    /// only passes if a new module has (a) the same public functions, and (b) for existing resources,
    /// no layout change.
    /// Enforcement: The Move upgradability patterns have been manually audited.
    ///
    /// No.: 4
    /// Requirement: Package upgrades should abide by policy change rules. In particular, The new
    /// upgrade policy must be equal to or stricter when compared to the old one. The original
    /// upgrade policy must not be immutable. The new package must contain all modules contained
    /// in the old package.
    /// Criticality: Medium
    /// Implementation: A package may only be updated using the publish_package function when the
    /// check_upgradability function returns true.
    /// Enforcement: This is audited by a manual review of the check_upgradability patterns.
    ///
    /// No.: 5
    /// Requirement: The upgrade policy of a package must not exceed the strictness level imposed by
    /// its dependencies.
    /// Criticality: Medium
    /// Implementation: The upgrade_policy of a package may only be less than its dependencies
    /// throughout the upgrades. In addition, the native code properly restricts the use of
    /// dependencies outside the passed-in metadata.
    /// Enforcement: This has been manually audited.
    ///
    /// No.: 6
    /// Requirement: The extension for package metadata is currently unused.
    /// Criticality: Medium
    /// Implementation: The extension field in PackageMetadata should be unused.
    /// Enforcement: Data invariant on the extension field has been manually audited.
    ///
    /// No.: 7
    /// Requirement: The upgrade number of a package increases incrementally in a monotonic manner
    /// with each subsequent upgrade.
    /// Criticality: Low
    /// Implementation: On each upgrade of a particular package, the publish_package function
    /// updates the upgrade_number for that package.
    /// Enforcement: Post condition on upgrade_number has been manually audited.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
    }

    spec request_publish {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec request_publish_with_allowed_deps {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec schema AbortsIfPermissionedSigner {
        use velor_framework::permissioned_signer;
        s: signer;
        let perm = CodePublishingPermission {};
        aborts_if !permissioned_signer::spec_check_permission_exists(s, perm);
    }

    spec initialize(velor_framework: &signer, package_owner: &signer, metadata: PackageMetadata) {
        let velor_addr = signer::address_of(velor_framework);
        let owner_addr = signer::address_of(package_owner);
        aborts_if !system_addresses::is_velor_framework_address(velor_addr);

        ensures exists<PackageRegistry>(owner_addr);
    }

    spec publish_package(owner: &signer, pack: PackageMetadata, code: vector<vector<u8>>) {
        // TODO: Can't verify 'vector::enumerate' loop.
        pragma aborts_if_is_partial;
        let addr = signer::address_of(owner);
        modifies global<PackageRegistry>(addr);
        aborts_if pack.upgrade_policy.policy <= upgrade_policy_arbitrary().policy;
        // include AbortsIfPermissionedSigner { s: owner };
    }

    spec publish_package_txn {
        // TODO: Calls `publish_package`.
        pragma verify = false;
    }

    spec check_upgradability(old_pack: &PackageMetadata, new_pack: &PackageMetadata, new_modules: &vector<String>) {
        // TODO: Can't verify 'vector::enumerate' loop.
        pragma aborts_if_is_partial;
        aborts_if old_pack.upgrade_policy.policy >= upgrade_policy_immutable().policy;
        aborts_if !can_change_upgrade_policy_to(old_pack.upgrade_policy, new_pack.upgrade_policy);
    }

    spec check_dependencies(publish_address: address, pack: &PackageMetadata): vector<AllowedDep> {
        // TODO: Can't verify 'vector::enumerate' loop.
        pragma verify = false;
    }

    spec check_coexistence(old_pack: &PackageMetadata, new_modules: &vector<String>) {
        // TODO: Can't verify 'vector::enumerate' loop.
        pragma verify = false;
    }

    spec get_module_names(pack: &PackageMetadata): vector<String> {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] len(result) == len(pack.modules);
        ensures [abstract] forall i in 0..len(result): result[i] == pack.modules[i].name;
    }

    spec freeze_code_object(publisher: &signer, code_object: Object<PackageRegistry>) {
        // TODO: Can't verify 'vector::for_each_mut' loop.
        pragma aborts_if_is_partial;

        let code_object_addr = code_object.inner;
        aborts_if !exists<object::ObjectCore>(code_object_addr);
        aborts_if !exists<PackageRegistry>(code_object_addr);
        aborts_if !object::is_owner(code_object, signer::address_of(publisher));
        // include AbortsIfPermissionedSigner { s: publisher };

        modifies global<PackageRegistry>(code_object_addr);
    }
}
