spec aptos_framework::code {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec request_publish {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec request_publish_with_allowed_deps {
        // TODO: temporary mockup.
        pragma opaque;
    }

    spec initialize(aptos_framework: &signer, package_owner: &signer, metadata: PackageMetadata) {
        let aptos_addr = signer::address_of(aptos_framework);
        let owner_addr = signer::address_of(package_owner);
        aborts_if !system_addresses::is_aptos_framework_address(aptos_addr);

        ensures exists<PackageRegistry>(owner_addr);
    }

    spec publish_package(owner: &signer, pack: PackageMetadata, code: vector<vector<u8>>) {
        // TODO: Can't verify 'vector::enumerate' loop.
        pragma aborts_if_is_partial;
        let addr = signer::address_of(owner);
        modifies global<PackageRegistry>(addr);
        aborts_if pack.upgrade_policy.policy <= upgrade_policy_arbitrary().policy;
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
}
