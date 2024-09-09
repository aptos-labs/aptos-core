/// A _permissioned signer_ consists of a pair of the original signer and a generated
/// signer which is used store information about associated permissions.
///
/// A permissioned signer behaves compatible with the original signer as it comes to `move_to`, `address_of`, and
/// existing basic signer functionality. However, the permissions can be queried to assert additional
/// restrictions on the use of the signer.
///
/// A client which is interested in restricting access granted via a signer can create a permissioned signer
/// and pass on to other existing code without changes to existing APIs. Core functions in the framework, for
/// example account functions, can then assert availability of permissions, effectively restricting
/// existing code in a compatible way.
///
/// After introducing the core functionality, examples are provided for withdraw limit on accounts, and
/// for blind signing.
module aptos_framework::permissioned_signer {
    use std::signer;
    use std::error;
    use std::vector;
    use std::option::{Option, Self};
    use aptos_std::copyable_any::{Self, Any};
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_framework::transaction_context::generate_auid_address;
    use aptos_framework::timestamp;

    #[test_only]
    friend aptos_framework::permissioned_signer_tests;

    /// Trying to grant permission using master signer.
    const ENOT_MASTER_SIGNER: u64 = 1;

    /// Cannot authorize a permission.
    const ECANNOT_AUTHORIZE: u64 = 2;

    /// Access permission information from a master signer.
    const ENOT_PERMISSIONED_SIGNER: u64 = 3;

    /// signer doesn't have enough capacity to extract permission.
    const ECANNOT_EXTRACT_PERMISSION: u64 = 4;

    /// permission handle has expired.
    const E_PERMISSION_EXPIRED: u64 = 5;

    /// storing extracted permission into a different signer.
    const E_PERMISSION_MISMATCH: u64 = 6;

    /// permission handle has been revoked by the original signer.
    const E_PERMISSION_REVOKED: u64 = 7;

    struct GrantedPermissionHandles has key {
        active_handles: vector<address>,
        revoked_handles: vector<address>,
    }

    struct PermissionedHandle {
        master_addr: address,
        permission_addr: address,
    }

    struct StorablePermissionedHandle has store {
        master_addr: address,
        permission_addr: address,
        expiration_time: u64,
    }

    struct PermStorage has key {
        perms: SmartTable<Any, u256>,
    }

    struct Permission<K> {
        owner_address: address,
        key: K,
        capacity: u256,
    }

    public fun create_permissioned_handle(master: &signer): PermissionedHandle {
        assert_master_signer(master);
        let permission_addr = generate_auid_address();
        let master_addr = signer::address_of(master);

        PermissionedHandle {
            master_addr,
            permission_addr,
        }
    }

    public(friend) fun create_storable_permissioned_handle(master: &signer, expiration_time: u64): StorablePermissionedHandle acquires GrantedPermissionHandles {
        assert_master_signer(master);
        let permission_addr = generate_auid_address();
        let master_addr = signer::address_of(master);

        if(!exists<GrantedPermissionHandles>(master_addr)) {
            move_to<GrantedPermissionHandles>(master, GrantedPermissionHandles {
                active_handles: vector::empty(),
                revoked_handles: vector::empty(),
            });
        };

        vector::push_back(
            &mut borrow_global_mut<GrantedPermissionHandles>(master_addr).active_handles,
            permission_addr
        );

        // Do we need to move sth similar to ObjectCore to register this address as permission address?
        StorablePermissionedHandle {
            master_addr,
            permission_addr,
            expiration_time,
        }
    }

    public fun destroy_permissioned_handle(p: PermissionedHandle) acquires PermStorage {
        let PermissionedHandle { master_addr: _, permission_addr } = p;
        if(exists<PermStorage>(permission_addr)) {
            let PermStorage { perms } = move_from<PermStorage>(permission_addr);
            smart_table::destroy(perms);
        };
    }

    public fun destroy_storable_permissioned_handle(p: StorablePermissionedHandle) acquires PermStorage, GrantedPermissionHandles {
        let StorablePermissionedHandle { master_addr, permission_addr, expiration_time: _ } = p;
        if(exists<PermStorage>(permission_addr)) {
            let PermStorage { perms } = move_from<PermStorage>(permission_addr);
            smart_table::destroy(perms);
        };
        let granted_permissions = borrow_global_mut<GrantedPermissionHandles>(master_addr);
        let (found, idx) = vector::index_of(&granted_permissions.active_handles, &permission_addr);
        if(found) {
            vector::swap_remove(&mut granted_permissions.active_handles, idx);
        };
        let (found, idx) = vector::index_of(&granted_permissions.revoked_handles, &permission_addr);
        if(found) {
            vector::swap_remove(&mut granted_permissions.revoked_handles, idx);
        };
    }


    public fun signer_from_permissioned(p: &PermissionedHandle): signer {
        signer_from_permissioned_impl(p.master_addr, p.permission_addr)
    }

    public fun signer_from_storable_permissioned(p: &StorablePermissionedHandle): signer acquires GrantedPermissionHandles {
        assert!(timestamp::now_seconds() < p.expiration_time, error::permission_denied(E_PERMISSION_EXPIRED));
        assert!(
            !vector::contains(
                &borrow_global<GrantedPermissionHandles>(p.master_addr).revoked_handles,
                &p.permission_addr
            ),
            error::permission_denied(E_PERMISSION_REVOKED)
        );
        signer_from_permissioned_impl(p.master_addr, p.permission_addr)
    }

    public fun revoke_permission_handle(s: &signer, permission_addr: address) acquires GrantedPermissionHandles {
        assert!(!is_permissioned_signer(s), error::permission_denied(ENOT_MASTER_SIGNER));
        let master_addr = signer::address_of(s);
        if(!exists<GrantedPermissionHandles>(master_addr)) {
            return
        };
        let granted_permissions = borrow_global_mut<GrantedPermissionHandles>(master_addr);
        if(!vector::contains(&granted_permissions.revoked_handles, &permission_addr)) {
            vector::push_back(&mut granted_permissions.revoked_handles, permission_addr)
        }
    }

    public fun permission_address(p: &StorablePermissionedHandle): address {
        p.permission_addr
    }

    public fun assert_master_signer(s: &signer) {
        assert!(!is_permissioned_signer(s), error::permission_denied(ENOT_MASTER_SIGNER));
    }

    /// =====================================================================================================
    /// Permission Management
    ///

    /// Authorizes `permissioned` with the given permission. This requires to have access to the `master`
    /// signer.
    public fun authorize<PermKey: copy + drop + store>(
        master: &signer,
        permissioned: &signer,
        capacity: u256,
        perm: PermKey
    ) acquires PermStorage {
        assert!(
            is_permissioned_signer(permissioned) &&
            !is_permissioned_signer(master) &&
            signer::address_of(master) == signer::address_of(permissioned),
            error::permission_denied(ECANNOT_AUTHORIZE)
        );
        let permission_signer = permission_signer(permissioned);
        let permission_signer_addr = signer::address_of(&permission_signer);
        if(!exists<PermStorage>(permission_signer_addr)) {
            move_to(&permission_signer, PermStorage { perms: smart_table::new()});
        };
        let perms = &mut borrow_global_mut<PermStorage>(permission_signer_addr).perms;
        let key = copyable_any::pack(perm);
        if(smart_table::contains(perms, key)) {
            let entry = smart_table::borrow_mut(perms, key);
            *entry = *entry + capacity;
        } else {
            smart_table::add(perms, key, capacity);
        }
    }

    public fun check_permission_exists<PermKey: copy + drop + store>(
        s: &signer,
        perm: PermKey
    ): bool acquires PermStorage {
        if (!is_permissioned_signer(s)) {
            // master signer has all permissions
            return true
        };
        let addr = signer::address_of(&permission_signer(s));
        if(!exists<PermStorage>(addr)) {
            return false
        };
        smart_table::contains(&mut borrow_global_mut<PermStorage>(addr).perms, copyable_any::pack(perm))
    }

    public fun check_permission_capacity_above<PermKey: copy + drop + store>(
        s: &signer,
        threshold: u256,
        perm: PermKey
    ): bool acquires PermStorage {
        if (!is_permissioned_signer(s)) {
            // master signer has all permissions
            return true
        };
        let addr = signer::address_of(&permission_signer(s));
        if(!exists<PermStorage>(addr)) {
            return false
        };
        let key = copyable_any::pack(perm);
        let storage = &borrow_global<PermStorage>(addr).perms;
        if(!smart_table::contains(storage, key)) {
            return false
        };
        let perm = smart_table::borrow(storage, key);
        if(*perm > threshold) {
            true
        } else {
            false
        }
    }

    public fun check_permission_consume<PermKey: copy + drop + store>(
        s: &signer,
        threshold: u256,
        perm: PermKey
    ): bool acquires PermStorage {
        if (!is_permissioned_signer(s)) {
            // master signer has all permissions
            return true
        };
        let addr = signer::address_of(&permission_signer(s));
        if(!exists<PermStorage>(addr)) {
            return false
        };
        let key = copyable_any::pack(perm);
        let storage = &mut borrow_global_mut<PermStorage>(addr).perms;
        if(!smart_table::contains(storage, key)) {
            return false
        };
        let perm = smart_table::borrow_mut(storage, key);
        if(*perm >= threshold) {
            *perm = *perm - threshold;
            true
        } else {
            false
        }
    }

    public fun capacity<PermKey: copy + drop + store>(s: &signer, perm: PermKey): Option<u256> acquires PermStorage {
        assert!(is_permissioned_signer(s), error::permission_denied(ENOT_PERMISSIONED_SIGNER));
        let addr = signer::address_of(&permission_signer(s));
        if(!exists<PermStorage>(addr)) {
            return option::none()
        };
        let perm_storage = &borrow_global<PermStorage>(addr).perms;
        let key = copyable_any::pack(perm);
        if(smart_table::contains(perm_storage, key)) {
            option::some(*smart_table::borrow(&borrow_global<PermStorage>(addr).perms, key))
        } else {
            option::none()
        }
    }

    public fun revoke_permission<PermKey: copy + drop + store>(permissioned: &signer, perm: PermKey) acquires PermStorage {
        if(!is_permissioned_signer(permissioned)) {
            // Master signer has no permissions associated with it.
            return
        };
        let addr = signer::address_of(&permission_signer(permissioned));
        if(!exists<PermStorage>(addr)) {
            return
        };
        smart_table::remove(&mut borrow_global_mut<PermStorage>(addr).perms, copyable_any::pack(perm));
    }

    /// Another flavor of api to extract and store permissions
    public fun extract_permission<PermKey: copy + drop + store>(
        s: &signer,
        weight: u256,
        perm: PermKey
    ): Permission<PermKey> acquires PermStorage {
        assert!(check_permission_consume(s, weight, perm), error::permission_denied(ECANNOT_EXTRACT_PERMISSION));
        Permission {
            owner_address: signer::address_of(s),
            key: perm,
            capacity: weight,
        }
    }

    public fun get_key<PermKey>(perm: &Permission<PermKey>): &PermKey {
        &perm.key
    }

    public fun address_of<PermKey>(perm: &Permission<PermKey>): address {
        perm.owner_address
    }

    public fun consume_permission<PermKey: copy + drop + store>(
        perm: &mut Permission<PermKey>,
        weight: u256,
        perm_key: PermKey
    ): bool {
        if(perm.key != perm_key) {
            return false
        };
        if(perm.capacity >= weight) {
            perm.capacity = perm.capacity - weight;
            return true
        } else {
            return false
        }
    }

    public fun store_permission<PermKey: copy + drop + store>(
        s: &signer,
        perm: Permission<PermKey>
    ) acquires PermStorage {
        assert!(is_permissioned_signer(s), error::permission_denied(ENOT_PERMISSIONED_SIGNER));
        let Permission { key, capacity, owner_address } = perm;

        assert!(signer::address_of(s) == owner_address, error::permission_denied(E_PERMISSION_MISMATCH));

        let permission_signer = permission_signer(s);
        let permission_signer_addr = signer::address_of(&permission_signer);
        if(!exists<PermStorage>(permission_signer_addr)) {
            move_to(&permission_signer, PermStorage { perms: smart_table::new()});
        };
        let perms = &mut borrow_global_mut<PermStorage>(permission_signer_addr).perms;
        let key = copyable_any::pack(key);
        if(smart_table::contains(perms, key)) {
            let entry = smart_table::borrow_mut(perms, key);
            *entry = *entry + capacity;
        } else {
            smart_table::add(perms, key, capacity)
        }
    }

    // =====================================================================================================
    // Native Functions

    /// Creates a permissioned signer from an existing universal signer. The function aborts if the
    /// given signer is already a permissioned signer.
    ///
    /// The implementation of this function requires to extend the value representation for signers in the VM.
    ///
    /// Check whether this is a permissioned signer.
    public native fun is_permissioned_signer(s: &signer): bool;
    /// Return the signer used for storing permissions. Aborts if not a permissioned signer.
    native fun permission_signer(permissioned: &signer): signer;
    ///
    /// invariants:
    ///   signer::address_of(master) == signer::address_of(signer_from_permissioned(create_permissioned_handle(master))),
    ///
    native fun signer_from_permissioned_impl(master_addr: address, permission_addr: address): signer;
}
