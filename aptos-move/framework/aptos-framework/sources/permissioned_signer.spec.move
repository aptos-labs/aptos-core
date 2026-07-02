spec aptos_framework::permissioned_signer {

    spec module {
        pragma verify = true;
        axiom forall a: GrantedPermissionHandles:
            (
                forall i in 0..len(a.active_handles):
                    forall j in 0..len(a.active_handles):
                        i != j ==>
                            a.active_handles[i] != a.active_handles[j]
            );
    }

    spec is_permissioned_signer_impl(s: &signer): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == std::signer::spec_is_permissioned_signer_impl(s);
    }

    spec fun spec_is_permissioned_signer(s: signer): bool {
        use std::features;
        use std::features::PERMISSIONED_SIGNER;
        if (!features::spec_is_enabled(PERMISSIONED_SIGNER)) {
            false
        } else {
            std::signer::spec_is_permissioned_signer_impl(s)
        }
    }

    spec is_permissioned_signer(s: &signer): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_is_permissioned_signer(s);
    }

    spec fun spec_permission_address(s: signer): address;

    spec permission_address(permissioned: &signer): address {
        pragma opaque;
        aborts_if [abstract]!spec_is_permissioned_signer(permissioned);
        ensures [abstract] result == spec_permission_address(permissioned);
    }

    spec fun spec_signer_from_permissioned_handle_impl(
        master_account_addr: address, permissions_storage_addr: address
    ): signer;

    spec signer_from_permissioned_handle_impl(
        master_account_addr: address, permissions_storage_addr: address
    ): signer {
        pragma opaque;
        ensures [abstract] result
            == spec_signer_from_permissioned_handle_impl(
                master_account_addr, permissions_storage_addr
            );
    }

    spec create_permissioned_handle(master: &signer): PermissionedHandle {
        use aptos_framework::transaction_context;
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if spec_is_permissioned_signer(master);
        let permissions_storage_addr = transaction_context::spec_generate_unique_address();
        modifies global<PermissionStorage>(permissions_storage_addr);
        let master_account_addr = signer::address_of(master);
        ensures result.master_account_addr == master_account_addr;
        ensures result.permissions_storage_addr == permissions_storage_addr;
    }

    spec create_storable_permissioned_handle(master: &signer, expiration_time: u64): StorablePermissionedHandle {
        use aptos_framework::transaction_context;
        pragma opaque;
        pragma aborts_if_is_partial;
        aborts_if spec_is_permissioned_signer(master);
        let permissions_storage_addr = transaction_context::spec_generate_unique_address();
        modifies global<PermissionStorage>(permissions_storage_addr);
        let master_account_addr = signer::address_of(master);
        modifies global<GrantedPermissionHandles>(master_account_addr);
        ensures result.master_account_addr == master_account_addr;
        ensures result.permissions_storage_addr == permissions_storage_addr;
        ensures result.expiration_time == expiration_time;
        ensures vector::spec_contains(
            global<GrantedPermissionHandles>(master_account_addr).active_handles,
            permissions_storage_addr
        );
        ensures exists<GrantedPermissionHandles>(master_account_addr);
    }

    spec destroy_permissioned_handle(p: PermissionedHandle) {
        ensures !exists<PermissionStorage>(p.permissions_storage_addr);
    }

    spec destroy_storable_permissioned_handle(p: StorablePermissionedHandle) {
        ensures !exists<PermissionStorage>(p.permissions_storage_addr);
        let post granted_permissions = global<GrantedPermissionHandles>(
            p.master_account_addr
        );
    }

    spec revoke_permission_storage_address(s: &signer, permissions_storage_addr: address) {
        // aborts_if spec_is_permissioned_signer(s);
    }

    spec authorize_increase<PermKey: copy + drop + store>(
        master: &signer, permissioned: &signer, capacity: u256, perm: PermKey
    ) {
        pragma aborts_if_is_partial;
        aborts_if !spec_is_permissioned_signer(permissioned);
        aborts_if spec_is_permissioned_signer(master);
        aborts_if signer::address_of(permissioned) != signer::address_of(master);
        ensures exists<PermissionStorage>(
            spec_permission_address(permissioned)
        );
    }

    spec check_permission_exists<PermKey: copy + drop + store>(s: &signer, perm: PermKey): bool {
        use aptos_std::type_info;
        use std::bcs;
        use aptos_framework::big_ordered_map;
        pragma opaque;
        modifies global<PermissionStorage>(spec_permission_address(s));
        aborts_if spec_is_permissioned_signer(s)
            && !exists<PermissionStorage>(spec_permission_address(s));

        let addr = spec_permission_address(s);
        let pre_perms = global<PermissionStorage>(addr).perms;
        let key = aptos_std::copyable_any::Any {
            type_name: type_info::type_name<PermKey>(),
            data: bcs::serialize(perm)
        };
        let pre_has = big_ordered_map::spec_contains_key(pre_perms, key);
        let pre_stored = big_ordered_map::spec_get(pre_perms, key);
        ensures exists<PermissionStorage>(addr) == old(exists<PermissionStorage>(addr));

        ensures [concrete] !spec_is_permissioned_signer(s) ==> result == true;
        ensures [concrete] (spec_is_permissioned_signer(s) && !pre_has) ==> result == false;
        ensures [concrete] (spec_is_permissioned_signer(s)
                && pre_has
                && (pre_stored is StoredPermission::Unlimited))
            ==> result == true;
        ensures [concrete] (spec_is_permissioned_signer(s)
                && pre_has
                && !(pre_stored is StoredPermission::Unlimited))
            ==> (result == (pre_stored.0 >= 1));
        ensures [abstract] result == spec_check_permission_exists(s, perm);
        ensures [abstract] !spec_is_permissioned_signer(s) ==> result == true;
    }

    spec fun spec_check_permission_exists<PermKey: copy + drop + store>(s: signer, perm: PermKey): bool;

    spec check_permission_capacity_above<PermKey: copy + drop + store>(
        s: &signer, threshold: u256, perm: PermKey
    ): bool {
        modifies global<PermissionStorage>(spec_permission_address(s));
        let permissioned_signer_addr = spec_permission_address(s);
        ensures !spec_is_permissioned_signer(s) ==> result == true;
        ensures (
            spec_is_permissioned_signer(s)
                && !exists<PermissionStorage>(permissioned_signer_addr)
        ) ==> result == false;
    }

    spec check_permission_consume<PermKey: copy + drop + store>(
        s: &signer, threshold: u256, perm: PermKey
    ): bool {
        use aptos_std::type_info;
        use std::bcs;
        use aptos_framework::big_ordered_map;
        pragma opaque;
        modifies global<PermissionStorage>(spec_permission_address(s));
        aborts_if spec_is_permissioned_signer(s)
            && !exists<PermissionStorage>(spec_permission_address(s));

        let addr = spec_permission_address(s);
        let pre_perms = global<PermissionStorage>(addr).perms;
        let key = aptos_std::copyable_any::Any {
            type_name: type_info::type_name<PermKey>(),
            data: bcs::serialize(perm)
        };
        let pre_has = big_ordered_map::spec_contains_key(pre_perms, key);
        let pre_stored = big_ordered_map::spec_get(pre_perms, key);

        ensures [concrete] !spec_is_permissioned_signer(s) ==> result == true;
        ensures [concrete] (spec_is_permissioned_signer(s) && !pre_has) ==> result == false;
        ensures [concrete] (spec_is_permissioned_signer(s)
                && pre_has
                && (pre_stored is StoredPermission::Unlimited))
            ==> result == true;
        ensures [concrete] (spec_is_permissioned_signer(s)
                && pre_has
                && !(pre_stored is StoredPermission::Unlimited))
            ==> (result == (pre_stored.0 >= threshold));
        ensures [abstract] result == spec_check_permission_consume(s, threshold, perm);
    }

    spec fun spec_check_permission_consume<PermKey: copy + drop + store>(s: signer, threshold: u256, perm: PermKey): bool;

    spec capacity<PermKey: copy + drop + store>(s: &signer, perm: PermKey): Option<u256> {
        use aptos_std::type_info;
        use std::bcs;
        use aptos_framework::big_ordered_map;
        pragma opaque;
        modifies global<PermissionStorage>(spec_permission_address(s));
        aborts_if spec_is_permissioned_signer(s)
            && !exists<PermissionStorage>(spec_permission_address(s));

        let addr = spec_permission_address(s);
        let pre_perms = global<PermissionStorage>(addr).perms;
        let key = aptos_std::copyable_any::Any {
            type_name: type_info::type_name<PermKey>(),
            data: bcs::serialize(perm)
        };
        let pre_has = big_ordered_map::spec_contains_key(pre_perms, key);
        let pre_stored = big_ordered_map::spec_get(pre_perms, key);

        ensures [concrete] !spec_is_permissioned_signer(s) ==>
            result == std::option::spec_some(MAX_U256);
        ensures [concrete] (spec_is_permissioned_signer(s) && !pre_has) ==>
            result == std::option::spec_none<u256>();
        ensures [concrete] (spec_is_permissioned_signer(s)
                && pre_has
                && (pre_stored is StoredPermission::Unlimited))
            ==> result == std::option::spec_some(MAX_U256);
        ensures [concrete] (spec_is_permissioned_signer(s)
                && pre_has
                && !(pre_stored is StoredPermission::Unlimited))
            ==> result == std::option::spec_some(pre_stored.0);
        ensures [abstract] result == spec_capacity(s, perm);
    }

    spec fun spec_capacity<PermKey: copy + drop + store>(s: signer, perm: PermKey): Option<u256>;
}
