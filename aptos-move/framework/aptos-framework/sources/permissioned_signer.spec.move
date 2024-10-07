spec aptos_framework::permissioned_signer {

    spec module {
        pragma verify = false;
        axiom forall a: GrantedPermissionHandles:
            (
                forall i in 0..len(a.active_handles):
                    forall j in 0..len(a.active_handles):
                        i != j ==>
                            a.active_handles[i] != a.active_handles[j]
            );
    }

    spec fun spec_is_permissioned_signer(s: signer): bool;

    spec is_permissioned_signer(s: &signer): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_permissioned_signer(s);
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
        aborts_if [abstract] spec_is_permissioned_signer(master);
        let permissions_storage_addr = transaction_context::spec_generate_unique_address();
        modifies global<PermissionStorage>(permissions_storage_addr);
        let master_account_addr = signer::address_of(master);
        ensures result.master_account_addr == master_account_addr;
        ensures result.permissions_storage_addr == permissions_storage_addr;
    }

    spec create_storable_permissioned_handle(master: &signer, expiration_time: u64): StorablePermissionedHandle {
        use aptos_framework::transaction_context;
        pragma opaque;
        aborts_if [abstract] spec_is_permissioned_signer(master);
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
        // ensures [abstract] !vector::spec_contains(granted_permissions.active_handles, p.permissions_storage_addr);
    }

    spec revoke_permission_storage_address(s: &signer, permissions_storage_addr: address) {
        // aborts_if spec_is_permissioned_signer(s);
    }

    spec authorize<PermKey: copy + drop + store>(
        master: &signer, permissioned: &signer, capacity: u256, perm: PermKey
    ) {

        // use aptos_std::type_info;
        // use std::bcs;
        pragma aborts_if_is_partial;
        aborts_if !spec_is_permissioned_signer(permissioned);
        aborts_if spec_is_permissioned_signer(master);
        aborts_if signer::address_of(permissioned) != signer::address_of(master);
        ensures exists<PermissionStorage>(
            spec_permission_address(permissioned)
        );
        // let perms = global<PermissionStorage>(permission_signer_addr).perms;
        // let post post_perms = global<PermissionStorage>(permission_signer_addr).perms;
        // let key = Any {
        //     type_name: type_info::type_name<SmartTable<Any, u256>>(),
        //     data: bcs::serialize(perm)
        // };
        // ensures smart_table::spec_contains(perms, key) ==>
        //     smart_table::spec_get(post_perms, key) == old(smart_table::spec_get(perms, key)) + capacity;
        // ensures !smart_table::spec_contains(perms, key) ==>
        //     smart_table::spec_get(post_perms, key) == capacity;
    }

    spec check_permission_exists<PermKey: copy + drop + store>(s: &signer, perm: PermKey): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_check_permission_exists(s, perm);
    }

    spec fun spec_check_permission_exists<PermKey: copy + drop + store>(s: signer, perm: PermKey): bool {
        use aptos_std::type_info;
        use std::bcs;
        let addr = spec_permission_address(s);
        let key = Any {
            type_name: type_info::type_name<PermKey>(),
            data: bcs::serialize(perm)
        };
        if (!spec_is_permissioned_signer(s)) { true }
        else if (!exists<PermissionStorage>(addr)) { false }
        else {
            simple_map::spec_contains_key(global<PermissionStorage>(addr).perms, key)
        }
    }

    spec check_permission_capacity_above<PermKey: copy + drop + store>(
        s: &signer, threshold: u256, perm: PermKey
    ): bool {
        use aptos_std::type_info;
        use std::bcs;
        let permissioned_signer_addr = spec_permission_address(s);
        ensures !spec_is_permissioned_signer(s) ==> result == true;
        ensures (
            spec_is_permissioned_signer(s)
                && !exists<PermissionStorage>(permissioned_signer_addr)
        ) ==> result == false;
        let key = Any {
            type_name: type_info::type_name<SimpleMap<Any, u256>>(),
            data: bcs::serialize(perm)
        };
        // ensures (spec_is_permissioned_signer(s) && exists<PermissionStorage>(permissioned_signer_addr) && !smart_table::spec_contains(global<PermissionStorage>(permissioned_signer_addr).perms, key)) ==>
        //     result == false;
        // ensures (spec_is_permissioned_signer(s) && exists<PermissionStorage>(permissioned_signer_addr) && smart_table::spec_contains(global<PermissionStorage>(permissioned_signer_addr).perms, key)) ==>
        //     result == (smart_table::spec_get(global<PermissionStorage>(permissioned_signer_addr).perms, key) > threshold);
    }

    spec check_permission_consume<PermKey: copy + drop + store>(
        s: &signer, threshold: u256, perm: PermKey
    ): bool {
        let permissioned_signer_addr = spec_permission_address(s);
        ensures !spec_is_permissioned_signer(s) ==> result == true;
        ensures (
            spec_is_permissioned_signer(s)
                && !exists<PermissionStorage>(permissioned_signer_addr)
        ) ==> result == false;

    }

    spec capacity<PermKey: copy + drop + store>(s: &signer, perm: PermKey): Option<u256> {
        // let permissioned_signer_addr = signer::address_of(spec_permission_address(s));
        // ensures !exists<PermissionStorage>(permissioned_signer_addr) ==>
        //     option::is_none(result);
    }

    spec consume_permission<PermKey: copy + drop + store>(
        perm: &mut Permission<PermKey>, weight: u256, perm_key: PermKey
    ): bool {
        // ensures perm.key != perm_key ==> result == false;
        // ensures perm.key == perm_key && old(perm.capacity) < weight ==> result == false;
        // ensures perm.key == perm_key
        //     && perm.capacity >= weight ==>
        //     (perm.capacity == old(perm.capacity) - weight
        //         && result == true);
    }
}
