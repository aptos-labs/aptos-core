/// A _permissioned signer_ consists of a pair of the original signer and a generated
/// address which is used store information about associated permissions.
///
/// A permissioned signer is a restricted version of a signer. Functions `move_to` and
/// `address_of` behave the same, and can be passed wherever signer is needed. However,
/// code can internally query for the permissions to assert additional restrictions on
/// the use of the signer.
///
/// A client which is interested in restricting access granted via a signer can create a permissioned signer
/// and pass on to other existing code without changes to existing APIs. Core functions in the framework, for
/// example account functions, can then assert availability of permissions, effectively restricting
/// existing code in a compatible way.
///
/// After introducing the core functionality, examples are provided for withdraw limit on accounts, and
/// for blind signing.
module aptos_framework::permissioned_signer {
    use std::features;
    use std::signer;
    use std::error;
    use std::vector;
    use std::option::{Option, Self};
    use aptos_std::copyable_any::{Self, Any};
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_framework::create_signer::create_signer;
    use aptos_framework::transaction_context::generate_auid_address;
    use aptos_framework::timestamp;

    friend aptos_framework::account;
    friend aptos_framework::aptos_governance;
    friend aptos_framework::code;
    friend aptos_framework::coin;
    friend aptos_framework::fungible_asset;
    friend aptos_framework::object;
    friend aptos_framework::stake;
    friend aptos_framework::staking_proxy;
    friend aptos_framework::voting;

    #[test_only]
    friend aptos_framework::permissioned_signer_tests;

    /// Trying to grant permission using non-master signer.
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

    /// destroying permission handle that has already been revoked or not owned by the
    /// given master signer.
    const E_NOT_ACTIVE: u64 = 8;

    /// Permissioned signer feature is not activated.
    const EPERMISSION_SIGNER_DISABLED: u64 = 9;

    const U256_MAX: u256 =
        115792089237316195423570985008687907853269984665640564039457584007913129639935;

    struct RevokePermissionHandlePermission has copy, store, drop {}

    struct GrantedPermissionHandles has key {
        active_handles: vector<address>
    }

    enum PermissionedHandle {
        V1 {
            master_account_addr: address,
            permissions_storage_addr: address
        }
    }

    enum StorablePermissionedHandle has store {
        V1 {
            master_account_addr: address,
            permissions_storage_addr: address,
            expiration_time: u64
        }
    }

    enum PermissionStorage has key {
        V1 {
            perms: SimpleMap<Any, StoredPermission>
        }
    }

    enum StoredPermission has store, copy, drop {
        Unlimited,
        Capacity(u256),
    }

    enum Permission<K> {
        V1 {
            owner_address: address,
            key: K,
            perm: StoredPermission,
        }
    }

    /// Create an ephermeral permission handle based on the master signer.
    ///
    /// This handle can be used to derive a signer that can be used in the context of
    /// the current transaction.
    public fun create_permissioned_handle(master: &signer): PermissionedHandle {
        assert_master_signer(master);
        let permissions_storage_addr = generate_auid_address();
        let master_account_addr = signer::address_of(master);

        move_to(
            &create_signer(permissions_storage_addr),
            PermissionStorage::V1 { perms: simple_map::new() }
        );

        PermissionedHandle::V1 { master_account_addr, permissions_storage_addr }
    }

    /// Create an storable permission handle based on the master signer.
    ///
    /// This handle can be used to derive a signer that can be stored by a smart contract.
    /// This is as dangerous as key delegation, thus it remains public(friend) for now.
    ///
    /// The caller should check if `expiration_time` is not too far in the future.
    public(friend) fun create_storable_permissioned_handle(
        master: &signer, expiration_time: u64
    ): StorablePermissionedHandle acquires GrantedPermissionHandles {
        assert_master_signer(master);
        let permissions_storage_addr = generate_auid_address();
        let master_account_addr = signer::address_of(master);

        assert!(
            timestamp::now_seconds() < expiration_time,
            error::permission_denied(E_PERMISSION_EXPIRED)
        );

        if (!exists<GrantedPermissionHandles>(master_account_addr)) {
            move_to<GrantedPermissionHandles>(
                master, GrantedPermissionHandles { active_handles: vector::empty() }
            );
        };

        vector::push_back(
            &mut borrow_global_mut<GrantedPermissionHandles>(master_account_addr).active_handles,
            permissions_storage_addr
        );

        move_to(
            &create_signer(permissions_storage_addr),
            PermissionStorage::V1 { perms: simple_map::new() }
        );

        StorablePermissionedHandle::V1 {
            master_account_addr,
            permissions_storage_addr,
            expiration_time
        }
    }

    /// Destroys an ephermeral permission handle. Clean up the permission stored in that handle
    public fun destroy_permissioned_handle(p: PermissionedHandle) acquires PermissionStorage {
        let PermissionedHandle::V1 { master_account_addr: _, permissions_storage_addr } =
            p;
        destroy_permissions_storage_address(permissions_storage_addr);
    }

    /// Destroys a storable permission handle. Clean up the permission stored in that handle
    public(friend) fun destroy_storable_permissioned_handle(
        p: StorablePermissionedHandle
    ) acquires PermissionStorage, GrantedPermissionHandles {
        let StorablePermissionedHandle::V1 {
            master_account_addr,
            permissions_storage_addr,
            expiration_time: _
        } = p;

        assert!(
            exists<GrantedPermissionHandles>(master_account_addr),
            error::permission_denied(E_PERMISSION_REVOKED),
        );
        let granted_permissions =
            borrow_global_mut<GrantedPermissionHandles>(master_account_addr);
        let (found, idx) = vector::index_of(
            &granted_permissions.active_handles, &permissions_storage_addr
        );

        // Removing the address from the active handle list if it's still active.
        if(found) {
            vector::swap_remove(&mut granted_permissions.active_handles, idx);
        };

        destroy_permissions_storage_address(permissions_storage_addr);
    }

    inline fun destroy_permissions_storage_address(
        permissions_storage_addr: address
    ) acquires PermissionStorage {
        if (exists<PermissionStorage>(permissions_storage_addr)) {
            let PermissionStorage::V1 { perms } =
                move_from<PermissionStorage>(permissions_storage_addr);
            simple_map::destroy(
                perms,
                |_dk| {},
                |_dv| {}
            );
        }
    }

    /// Generate the permissioned signer based on the ephermeral permission handle.
    ///
    /// This signer can be used as a regular signer for other smart contracts. However when such
    /// signer interacts with various framework functions, it would subject to permission checks
    /// and would abort if check fails.
    public fun signer_from_permissioned_handle(p: &PermissionedHandle): signer {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );
        signer_from_permissioned_handle_impl(
            p.master_account_addr, p.permissions_storage_addr
        )
    }

    /// Generate the permissioned signer based on the storable permission handle.
    public(friend) fun signer_from_storable_permissioned_handle(
        p: &StorablePermissionedHandle
    ): signer {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );
        assert!(
            timestamp::now_seconds() < p.expiration_time,
            error::permission_denied(E_PERMISSION_EXPIRED)
        );
        assert!(
            exists<PermissionStorage>(p.permissions_storage_addr),
            error::permission_denied(E_PERMISSION_REVOKED)
        );
        signer_from_permissioned_handle_impl(
            p.master_account_addr, p.permissions_storage_addr
        )
    }

    public fun grant_revoke_permission(
        master: &signer,
        permissioned: &signer,
    ) acquires PermissionStorage {
        authorize_unlimited(master, permissioned, RevokePermissionHandlePermission {});
    }

    /// Revoke a specific storable permission handle immediately. This would disallow owner of
    /// the storable permission handle to derive signer from it anymore.
    public entry fun revoke_permission_storage_address(
        s: &signer, permissions_storage_addr: address
    ) acquires GrantedPermissionHandles, PermissionStorage {
        assert!(
            check_permission_exists(s, RevokePermissionHandlePermission {}),
            error::permission_denied(ENOT_MASTER_SIGNER)
        );
        let master_account_addr = signer::address_of(s);

        assert!(
            exists<GrantedPermissionHandles>(master_account_addr),
            error::permission_denied(E_PERMISSION_REVOKED),
        );
        let granted_permissions =
            borrow_global_mut<GrantedPermissionHandles>(master_account_addr);
        let (found, idx) = vector::index_of(
            &granted_permissions.active_handles, &permissions_storage_addr
        );

        // The address has to be in the activated list in the master account address.
        assert!(found, error::permission_denied(E_NOT_ACTIVE));
        vector::swap_remove(&mut granted_permissions.active_handles, idx);
        destroy_permissions_storage_address(permissions_storage_addr);
    }

    /// Revoke all storable permission handle of the signer immediately.
    public entry fun revoke_all_handles(s: &signer) acquires GrantedPermissionHandles, PermissionStorage {
        assert!(
            check_permission_exists(s, RevokePermissionHandlePermission {}),
            error::permission_denied(ENOT_MASTER_SIGNER)
        );
        let master_account_addr = signer::address_of(s);
        if (!exists<GrantedPermissionHandles>(master_account_addr)) { return };

        let granted_permissions =
            borrow_global_mut<GrantedPermissionHandles>(master_account_addr);
        let delete_list = vector::trim_reverse(
            &mut granted_permissions.active_handles, 0
        );
        vector::destroy(
            delete_list,
            |address| {
                destroy_permissions_storage_address(address);
            }
        )
    }

    /// Return the permission handle address so that it could be used for revocation purpose.
    public(friend) fun permissions_storage_address(
        p: &StorablePermissionedHandle
    ): address {
        p.permissions_storage_addr
    }

    /// Helper function that would abort if the signer passed in is a permissioned signer.
    public(friend) fun assert_master_signer(s: &signer) {
        assert!(
            !is_permissioned_signer(s), error::permission_denied(ENOT_MASTER_SIGNER)
        );
    }

    /// =====================================================================================================
    /// StoredPermission operations
    ///
    /// check if StoredPermission has at least `threshold` capacity.
    fun is_above(perm: &StoredPermission, threshold: u256): bool {
        match (perm) {
            StoredPermission::Capacity(capacity) => *capacity > threshold,
            StoredPermission::Unlimited => true,
        }
    }

    /// consume `threshold` capacity from StoredPermission
    fun consume_capacity(perm: &mut StoredPermission, threshold: u256): bool {
        match (perm) {
            StoredPermission::Capacity(current_capacity) => {
                if (*current_capacity >= threshold) {
                    *current_capacity = *current_capacity - threshold;
                    true
                } else { false }
            }
            StoredPermission::Unlimited => true
        }
    }

    /// increase `threshold` capacity from StoredPermission
    fun increase_capacity(perm: &mut StoredPermission, threshold: u256) {
        match (perm) {
            StoredPermission::Capacity(current_capacity) => {
                *current_capacity = *current_capacity + threshold;
            }
            StoredPermission::Unlimited => (),
        }
    }

    /// merge the two stored permission
    fun merge(lhs: &mut StoredPermission, rhs: StoredPermission) {
        match (rhs) {
            StoredPermission::Capacity(new_capacity) => {
                match (lhs) {
                    StoredPermission::Capacity(current_capacity) => {
                        *current_capacity = *current_capacity + new_capacity;
                    }
                    StoredPermission::Unlimited => (),
                }
            }
            StoredPermission::Unlimited => *lhs = StoredPermission::Unlimited,
        }
    }

    /// =====================================================================================================
    /// Permission Management
    ///
    /// Authorizes `permissioned` with the given permission. This requires to have access to the `master`
    /// signer.

    inline fun map_or<PermKey: copy + drop + store, T>(
        permissioned: &signer,
        perm: PermKey,
        mutate: |&mut StoredPermission| T,
        default: T,
    ): T {
        let permission_signer_addr = permission_address(permissioned);
        assert!(
            exists<PermissionStorage>(permission_signer_addr),
            error::permission_denied(E_NOT_ACTIVE)
        );
        let perms =
            &mut borrow_global_mut<PermissionStorage>(permission_signer_addr).perms;
        let key = copyable_any::pack(perm);
        if (simple_map::contains_key(perms, &key)) {
            mutate(simple_map::borrow_mut(perms, &key))
        } else {
            default
        }
    }

    inline fun insert_or<PermKey: copy + drop + store>(
        permissioned: &signer,
        perm: PermKey,
        mutate: |&mut StoredPermission|,
        default: StoredPermission,
    ) {
        let permission_signer_addr = permission_address(permissioned);
        assert!(
            exists<PermissionStorage>(permission_signer_addr),
            error::permission_denied(E_NOT_ACTIVE)
        );
        let perms =
            &mut borrow_global_mut<PermissionStorage>(permission_signer_addr).perms;
        let key = copyable_any::pack(perm);
        if (simple_map::contains_key(perms, &key)) {
            mutate(simple_map::borrow_mut(perms, &key));
        } else {
            simple_map::add(perms, key, default);
        }
    }

    /// Authorizes `permissioned` with a given capacity and increment the existing capacity if present.
    ///
    /// Consumption using `check_permission_consume` will deduct the capacity.
    public fun authorize_increase<PermKey: copy + drop + store>(
        master: &signer,
        permissioned: &signer,
        capacity: u256,
        perm: PermKey
    ) acquires PermissionStorage {
        assert!(
            is_permissioned_signer(permissioned)
                && !is_permissioned_signer(master)
                && signer::address_of(master) == signer::address_of(permissioned),
            error::permission_denied(ECANNOT_AUTHORIZE)
        );
        insert_or(
            permissioned,
            perm,
            |stored_permission| {
                increase_capacity(stored_permission, capacity);
            },
            StoredPermission::Capacity(capacity),
        )
    }

    /// Authorizes `permissioned` with the given unlimited permission.
    /// Unlimited permission can be consumed however many times.
    public fun authorize_unlimited<PermKey: copy + drop + store>(
        master: &signer,
        permissioned: &signer,
        perm: PermKey
    ) acquires PermissionStorage {
        assert!(
            is_permissioned_signer(permissioned)
                && !is_permissioned_signer(master)
                && signer::address_of(master) == signer::address_of(permissioned),
            error::permission_denied(ECANNOT_AUTHORIZE)
        );
        insert_or(
            permissioned,
            perm,
            |stored_permission| {
                *stored_permission = StoredPermission::Unlimited;
            },
            StoredPermission::Unlimited,
        )
    }

    /// Increase the `capacity` of a permissioned signer **without** master signer's approvoal.
    ///
    /// The caller of the module will need to make sure the witness type `PermKey` can only be
    /// constructed within its own module, otherwise attackers can refill the permission for itself
    /// to bypass the checks.
    public(friend) fun increase_limit<PermKey: copy + drop + store>(
        permissioned: &signer,
        capacity: u256,
        perm: PermKey
    ) acquires PermissionStorage {
        if(!is_permissioned_signer(permissioned)) {
            return;
        };
        insert_or(
            permissioned,
            perm,
            |stored_permission| {
                increase_capacity(stored_permission, capacity);
            },
            StoredPermission::Capacity(capacity),
        )
    }

    public fun check_permission_exists<PermKey: copy + drop + store>(
        s: &signer, perm: PermKey
    ): bool acquires PermissionStorage {
        check_permission_capacity_above(s, 0, perm)
    }

    public fun check_permission_capacity_above<PermKey: copy + drop + store>(
        s: &signer, threshold: u256, perm: PermKey
    ): bool acquires PermissionStorage {
        if (!is_permissioned_signer(s)) {
            // master signer has all permissions
            return true
        };
        map_or(
            s,
            perm,
            |stored_permission| {
                is_above(stored_permission, threshold)
            },
            false,
        )
    }

    public fun check_permission_consume<PermKey: copy + drop + store>(
        s: &signer, threshold: u256, perm: PermKey
    ): bool acquires PermissionStorage {
        if (!is_permissioned_signer(s)) {
            // master signer has all permissions
            return true
        };
        map_or(
            s,
            perm,
            |stored_permission| {
                 consume_capacity(stored_permission, threshold)
            },
            false,
        )
    }

    public(friend) fun capacity<PermKey: copy + drop + store>(
        s: &signer, perm: PermKey
    ): Option<u256> acquires PermissionStorage {
        if (!is_permissioned_signer(s)) {
            return option::some(U256_MAX)
        };
        map_or(
            s,
            perm,
            |stored_permission: &mut StoredPermission| {
                option::some(match (stored_permission) {
                    StoredPermission::Capacity(capacity) => *capacity,
                    StoredPermission::Unlimited => U256_MAX,
                })
            },
            option::none(),
        )
    }

    public(friend) fun revoke_permission<PermKey: copy + drop + store>(
        permissioned: &signer, perm: PermKey
    ) acquires PermissionStorage {
        if (!is_permissioned_signer(permissioned)) {
            // Master signer has no permissions associated with it.
            return
        };
        let addr = permission_address(permissioned);
        if (!exists<PermissionStorage>(addr)) { return };
        let perm_storage = &mut borrow_global_mut<PermissionStorage>(addr).perms;
        let key = copyable_any::pack(perm);
        if (simple_map::contains_key(perm_storage, &key)) {
            simple_map::remove(
                &mut borrow_global_mut<PermissionStorage>(addr).perms,
                &copyable_any::pack(perm)
            );
        }
    }

    /// =====================================================================================================
    /// Another flavor of api to extract and store permissions
    ///
    public(friend) fun extract_permission<PermKey: copy + drop + store>(
        s: &signer, weight: u256, perm: PermKey
    ): Permission<PermKey> acquires PermissionStorage {
        assert!(
            check_permission_consume(s, weight, perm),
            error::permission_denied(ECANNOT_EXTRACT_PERMISSION)
        );
        Permission::V1 {
            owner_address: signer::address_of(s),
            key: perm,
            perm: StoredPermission::Capacity(weight),
        }
    }

    public(friend) fun extract_all_permission<PermKey: copy + drop + store>(
        s: &signer, perm_key: PermKey
    ): Permission<PermKey> acquires PermissionStorage {
        assert!(
            is_permissioned_signer(s),
            error::permission_denied(ECANNOT_EXTRACT_PERMISSION)
        );
        let addr = permission_address(s);
        assert!(
            exists<PermissionStorage>(addr),
            error::permission_denied(ECANNOT_EXTRACT_PERMISSION)
        );
        let key = copyable_any::pack(perm_key);
        let storage = &mut borrow_global_mut<PermissionStorage>(addr).perms;
        let (_, value) = simple_map::remove(storage, &key);

        Permission::V1 {
            owner_address: signer::address_of(s),
            key: perm_key,
            perm: value,
        }
    }

    public(friend) fun address_of<PermKey>(perm: &Permission<PermKey>): address {
        perm.owner_address
    }

    public(friend) fun consume_permission<PermKey: copy + drop + store>(
        perm: &mut Permission<PermKey>, weight: u256, perm_key: PermKey
    ): bool {
        if (perm.key != perm_key) {
            return false
        };
        consume_capacity(&mut perm.perm, weight)
    }

    public(friend) fun store_permission<PermKey: copy + drop + store>(
        s: &signer, perm: Permission<PermKey>
    ) acquires PermissionStorage {
        assert!(
            is_permissioned_signer(s),
            error::permission_denied(ENOT_PERMISSIONED_SIGNER)
        );
        let Permission::V1 { key, perm, owner_address } = perm;

        assert!(
            signer::address_of(s) == owner_address,
            error::permission_denied(E_PERMISSION_MISMATCH)
        );

        insert_or(
            s,
            key,
            |stored_permission| {
                merge(stored_permission, perm);
            },
            perm,
        )
    }

    // =====================================================================================================
    // Native Functions
    ///
    /// Check whether this is a permissioned signer.
    public native fun is_permissioned_signer(s: &signer): bool;
    /// Return the address used for storing permissions. Aborts if not a permissioned signer.
    native fun permission_address(permissioned: &signer): address;
    /// Creates a permissioned signer from an existing universal signer. The function aborts if the
    /// given signer is already a permissioned signer.
    ///
    /// The implementation of this function requires to extend the value representation for signers in the VM.
    /// invariants:
    ///   signer::address_of(master) == signer::address_of(signer_from_permissioned_handle(create_permissioned_handle(master))),
    ///
    native fun signer_from_permissioned_handle_impl(
        master_account_addr: address, permissions_storage_addr: address
    ): signer;

    #[test(creator = @0xcafe)]
    fun signer_address_roundtrip(
        creator: &signer
    ) acquires PermissionStorage, GrantedPermissionHandles {
        let aptos_framework = create_signer(@0x1);
        timestamp::set_time_has_started_for_testing(&aptos_framework);

        let handle = create_permissioned_handle(creator);
        let perm_signer = signer_from_permissioned_handle(&handle);
        assert!(signer::address_of(&perm_signer) == signer::address_of(creator), 1);
        assert!(
            permission_address(&perm_signer)
                == handle.permissions_storage_addr,
            1
        );
        assert!(exists<PermissionStorage>(handle.permissions_storage_addr), 1);

        destroy_permissioned_handle(handle);

        let handle = create_storable_permissioned_handle(creator, 60);
        let perm_signer = signer_from_storable_permissioned_handle(&handle);
        assert!(signer::address_of(&perm_signer) == signer::address_of(creator), 1);
        assert!(
            permission_address(&perm_signer)
                == handle.permissions_storage_addr,
            1
        );
        assert!(exists<PermissionStorage>(handle.permissions_storage_addr), 1);

        destroy_storable_permissioned_handle(handle);
    }
}
