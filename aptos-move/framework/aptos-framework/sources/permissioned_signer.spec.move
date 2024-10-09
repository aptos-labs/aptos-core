spec aptos_framework::permissioned_signer {

    spec module {
        axiom forall a: GrantedPermissionHandles:
            (forall i in 0..len(a.active_handles):
                forall j in 0..len(a.active_handles):
                    i != j ==> a.active_handles[i] != a.active_handles[j]
            );
    }

    spec fun spec_is_permissioned_signer(s: signer): bool;

    spec is_permissioned_signer(s: &signer): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_permissioned_signer(s);
    }

    spec fun spec_permission_signer(s: signer): signer;

    spec permission_signer(permissioned: &signer): signer {
        pragma opaque;
        aborts_if [abstract] !spec_is_permissioned_signer(permissioned);
        ensures [abstract] result == spec_permission_signer(permissioned);
    }

    spec fun spec_signer_from_permissioned_impl(master_addr: address, permission_addr: address): signer;

    spec signer_from_permissioned_impl(master_addr: address, permission_addr: address): signer {
        pragma opaque;
        ensures [abstract] result == spec_signer_from_permissioned_impl(master_addr, permission_addr);
    }

    spec create_permissioned_handle(master: &signer): PermissionedHandle {
        use aptos_framework::transaction_context;
        pragma opaque;
        aborts_if [abstract] spec_is_permissioned_signer(master);
        let permission_addr = transaction_context::spec_generate_unique_address();
        modifies global<PermStorage>(permission_addr);
        let master_addr = signer::address_of(master);
        ensures result.master_addr == master_addr;
        ensures result.permission_addr == permission_addr;
    }

    spec create_storable_permissioned_handle(master: &signer, expiration_time: u64): StorablePermissionedHandle {
        use aptos_framework::transaction_context;
        pragma opaque;
        aborts_if [abstract] spec_is_permissioned_signer(master);
        let permission_addr = transaction_context::spec_generate_unique_address();
        modifies global<PermStorage>(permission_addr);
        let master_addr = signer::address_of(master);
        modifies global<GrantedPermissionHandles>(master_addr);
        ensures result.master_addr == master_addr;
        ensures result.permission_addr == permission_addr;
        ensures result.expiration_time == expiration_time;
        ensures vector::spec_contains(global<GrantedPermissionHandles>(master_addr).active_handles, permission_addr);
        ensures exists<GrantedPermissionHandles>(master_addr);
    }

    spec destroy_permissioned_handle(p: PermissionedHandle) {
        ensures !exists<PermStorage>(p.permission_addr);
    }

    spec destroy_storable_permissioned_handle(p: StorablePermissionedHandle) {
        ensures !exists<PermStorage>(p.permission_addr);
        let post granted_permissions = global<GrantedPermissionHandles>(p.master_addr);
        // ensures [abstract] !vector::spec_contains(granted_permissions.active_handles, p.permission_addr);
    }

    spec revoke_permission_handle(s: &signer, permission_addr: address) {
        aborts_if spec_is_permissioned_signer(s);
    }

    spec authorize<PermKey: copy + drop + store>(
    master: &signer,
    permissioned: &signer,
    capacity: u256,
    perm: PermKey
    ) {

        // use aptos_std::type_info;
        // use std::bcs;
        pragma aborts_if_is_partial;
        aborts_if !spec_is_permissioned_signer(permissioned);
        aborts_if spec_is_permissioned_signer(master);
        aborts_if signer::address_of(permissioned) != signer::address_of(master);
        ensures exists<PermStorage>(signer::address_of(spec_permission_signer(permissioned)));
        // let perms = global<PermStorage>(permission_signer_addr).perms;
        // let post post_perms = global<PermStorage>(permission_signer_addr).perms;
        // let key = Any {
        //     type_name: type_info::type_name<SmartTable<Any, u256>>(),
        //     data: bcs::serialize(perm)
        // };
        // ensures smart_table::spec_contains(perms, key) ==>
        //     smart_table::spec_get(post_perms, key) == old(smart_table::spec_get(perms, key)) + capacity;
        // ensures !smart_table::spec_contains(perms, key) ==>
        //     smart_table::spec_get(post_perms, key) == capacity;
    }

    spec check_permission_exists<PermKey: copy + drop + store>(
    s: &signer,
    perm: PermKey
    ): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_check_permission_exists(s, perm);
    }

    spec fun spec_check_permission_exists<PermKey: copy + drop + store>(
        s: signer,
        perm: PermKey
    ): bool {
        use aptos_std::type_info;
        use std::bcs;
        let addr = signer::address_of(spec_permission_signer(s));
        let key = Any {
            type_name: type_info::type_name<PermKey>(),
            data: bcs::serialize(perm)
        };
        if (!spec_is_permissioned_signer(s)) {
            true
        } else if(!exists<PermStorage>(addr)) {
            false
        } else {
            smart_table::spec_contains(global<PermStorage>(addr).perms, key)
        }
    }

    spec check_permission_capacity_above<PermKey: copy + drop + store>(
    s: &signer,
    threshold: u256,
    perm: PermKey
    ): bool {
        use aptos_std::type_info;
        use std::bcs;
        let permissioned_signer_addr = signer::address_of(spec_permission_signer(s));
        ensures !spec_is_permissioned_signer(s) ==> result == true;
        ensures (spec_is_permissioned_signer(s) && !exists<PermStorage>(permissioned_signer_addr)) ==> result == false;
        let key = Any {
            type_name: type_info::type_name<SmartTable<Any, u256>>(),
            data: bcs::serialize(perm)
        };
        // ensures (spec_is_permissioned_signer(s) && exists<PermStorage>(permissioned_signer_addr) && !smart_table::spec_contains(global<PermStorage>(permissioned_signer_addr).perms, key)) ==>
        //     result == false;
        // ensures (spec_is_permissioned_signer(s) && exists<PermStorage>(permissioned_signer_addr) && smart_table::spec_contains(global<PermStorage>(permissioned_signer_addr).perms, key)) ==>
        //     result == (smart_table::spec_get(global<PermStorage>(permissioned_signer_addr).perms, key) > threshold);
    }

    spec check_permission_consume<PermKey: copy + drop + store>(
    s: &signer,
    threshold: u256,
    perm: PermKey
    ): bool {
        let permissioned_signer_addr = signer::address_of(spec_permission_signer(s));
        ensures !spec_is_permissioned_signer(s) ==> result == true;
        ensures (spec_is_permissioned_signer(s) && !exists<PermStorage>(permissioned_signer_addr)) ==> result == false;

    }

    spec capacity<PermKey: copy + drop + store>(s: &signer, perm: PermKey): Option<u256> {
        aborts_if !spec_is_permissioned_signer(s);
        let permissioned_signer_addr = signer::address_of(spec_permission_signer(s));
        ensures !exists<PermStorage>(permissioned_signer_addr) ==> option::is_none(result);
    }

    spec consume_permission<PermKey: copy + drop + store>(
        perm: &mut Permission<PermKey>,
        weight: u256,
        perm_key: PermKey
    ): bool {
        ensures perm.key != perm_key ==> result == false;
        ensures perm.key == perm_key && old(perm.capacity) < weight ==> result == false;
        ensures perm.key == perm_key && perm.capacity >= weight ==>
            (perm.capacity == old(perm.capacity) - weight && result == true);
    }

}
