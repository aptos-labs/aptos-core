/// A _permissioned signer_ consists of a pair of the original signer and a generated
/// address which is used to store information about associated permissions.
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
module velor_framework::permissioned_signer {
    use std::features;
    use std::signer;
    use std::error;
    use std::vector;
    use std::option::{Option, Self};
    use velor_std::copyable_any::{Self, Any};
    use velor_framework::big_ordered_map::{Self, BigOrderedMap};
    use velor_framework::create_signer::create_signer;
    use velor_framework::transaction_context::generate_auid_address;
    use velor_framework::timestamp;

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

    /// If a permissioned signer has this permission, it would be able to revoke other granted
    /// permission handles in the same signer.
    struct RevokePermissionHandlePermission has copy, store, drop {}

    /// Stores the list of granted permission handles for a given account.
    struct GrantedPermissionHandles has key {
        /// Each address refers to a `permissions_storage_addr` that stores the `PermissionStorage`.
        active_handles: vector<address>
    }

    /// A ephermeral permission handle that can be used to generate a permissioned signer with permission
    /// configuration stored within.
    enum PermissionedHandle {
        V1 {
            /// Address of the signer that creates this handle.
            master_account_addr: address,
            /// Address that stores `PermissionStorage`.
            permissions_storage_addr: address
        }
    }

    /// A permission handle that can be used to generate a permissioned signer.
    ///
    /// This handle is storable and thus should be treated very carefully as it serves similar functionality
    /// as signer delegation.
    enum StorablePermissionedHandle has store {
        V1 {
            /// Address of the signer that creates this handle.
            master_account_addr: address,
            /// Address that stores `PermissionStorage`.
            permissions_storage_addr: address,
            /// Permissioned signer can no longer be generated from this handle after `expiration_time`.
            expiration_time: u64
        }
    }

    /// The actual permission configuration stored on-chain.
    ///
    /// The address that holds `PermissionStorage` will be generated freshly every time a permission
    /// handle gets created.
    enum PermissionStorage has key {
        V1 {
            /// A hetherogenous map from `Permission` structs defined by each different modules to
            /// its permission capacity.
            perms: BigOrderedMap<Any, StoredPermission>
        }
    }

    /// Types of permission capacity stored on chain.
    enum StoredPermission has store, copy, drop {
        /// Unlimited capacity.
        Unlimited,
        /// Fixed capacity, will be deducted when permission is used.
        Capacity(u256),
    }

    /// Create an ephermeral permission handle based on the master signer.
    ///
    /// This handle can be used to derive a signer that can be used in the context of
    /// the current transaction.
    public fun create_permissioned_handle(master: &signer): PermissionedHandle {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );

        assert_master_signer(master);
        let permissions_storage_addr = generate_auid_address();
        let master_account_addr = signer::address_of(master);

        initialize_permission_address(permissions_storage_addr);

        PermissionedHandle::V1 { master_account_addr, permissions_storage_addr }
    }

    /// Destroys an ephermeral permission handle. Clean up the permission stored in that handle
    public fun destroy_permissioned_handle(p: PermissionedHandle) acquires PermissionStorage {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );
        let PermissionedHandle::V1 { master_account_addr: _, permissions_storage_addr } =
            p;
        destroy_permissions_storage_address(permissions_storage_addr);
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

    /// Returns true if `s` is a permissioned signer.
    public fun is_permissioned_signer(s: &signer): bool {
        // When the permissioned signer is disabled, no one is able to construct a permissioned
        // signer. Thus we should return false here, as other on chain permission checks will
        // depend on this checks.
        if(!features::is_permissioned_signer_enabled()) {
            return false;
        };
        is_permissioned_signer_impl(s)
    }

    /// Grant the permissioned signer the permission to revoke granted permission handles under
    /// its address.
    public fun grant_revoke_permission(
        master: &signer,
        permissioned: &signer,
    ) acquires PermissionStorage {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );
        authorize_unlimited(master, permissioned, RevokePermissionHandlePermission {});
    }

    /// Revoke a specific storable permission handle immediately. This will disallow owner of
    /// the storable permission handle to derive signer from it anymore.
    public entry fun revoke_permission_storage_address(
        s: &signer, permissions_storage_addr: address
    ) acquires GrantedPermissionHandles, PermissionStorage {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );
        assert!(
            check_permission_exists(s, RevokePermissionHandlePermission {}),
            error::permission_denied(ENOT_MASTER_SIGNER)
        );
        let master_account_addr = signer::address_of(s);

        assert!(
            exists<GrantedPermissionHandles>(master_account_addr),
            error::permission_denied(E_PERMISSION_REVOKED),
        );
        let active_handles = &mut GrantedPermissionHandles[master_account_addr].active_handles;
        let (found, idx) = active_handles.index_of(&permissions_storage_addr);

        // The address has to be in the activated list in the master account address.
        assert!(found, error::permission_denied(E_NOT_ACTIVE));
        active_handles.swap_remove(idx);
        destroy_permissions_storage_address(permissions_storage_addr);
    }

    /// Revoke all storable permission handle of the signer immediately.
    public entry fun revoke_all_handles(s: &signer) acquires GrantedPermissionHandles, PermissionStorage {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );
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

    /// initialize permission storage by putting an empty storage under the address.
    inline fun initialize_permission_address(permissions_storage_addr: address) {
        move_to(
            &create_signer(permissions_storage_addr),
            // Each key is ~100bytes, the value is 12 bytes.
            PermissionStorage::V1 { perms: big_ordered_map::new_with_config(40, 35, false) }
        );
    }

    /// Create an storable permission handle based on the master signer.
    ///
    /// This handle can be used to derive a signer that can be stored by a smart contract.
    /// This is as dangerous as key delegation, thus it remains public(package) for now.
    ///
    /// The caller should check if `expiration_time` is not too far in the future.
    public(package) fun create_storable_permissioned_handle(
        master: &signer, expiration_time: u64
    ): StorablePermissionedHandle acquires GrantedPermissionHandles {
        assert!(
            features::is_permissioned_signer_enabled(),
            error::permission_denied(EPERMISSION_SIGNER_DISABLED)
        );

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

        GrantedPermissionHandles[master_account_addr]
            .active_handles.push_back(permissions_storage_addr);

        initialize_permission_address(permissions_storage_addr);

        StorablePermissionedHandle::V1 {
            master_account_addr,
            permissions_storage_addr,
            expiration_time
        }
    }

    /// Destroys a storable permission handle. Clean up the permission stored in that handle
    public(package) fun destroy_storable_permissioned_handle(
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
        let active_handles = &mut GrantedPermissionHandles[master_account_addr].active_handles;

        let (found, idx) = active_handles.index_of(&permissions_storage_addr);

        // Removing the address from the active handle list if it's still active.
        if(found) {
            active_handles.swap_remove(idx);
        };

        destroy_permissions_storage_address(permissions_storage_addr);
    }

    inline fun destroy_permissions_storage_address(permissions_storage_addr: address) {
        if (exists<PermissionStorage>(permissions_storage_addr)) {
            let PermissionStorage::V1 { perms } =
                move_from<PermissionStorage>(permissions_storage_addr);
            big_ordered_map::destroy(
                perms,
                |_dv| {},
            );
        }
    }

    /// Generate the permissioned signer based on the storable permission handle.
    public(package) fun signer_from_storable_permissioned_handle(
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

    /// Return the permission handle address so that it could be used for revocation purpose.
    public(package) fun permissions_storage_address(
        p: &StorablePermissionedHandle
    ): address {
        p.permissions_storage_addr
    }

    /// Helper function that would abort if the signer passed in is a permissioned signer.
    public(package) fun assert_master_signer(s: &signer) {
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
            StoredPermission::Capacity(capacity) => *capacity >= threshold,
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
        if (big_ordered_map::contains(perms, &key)) {
            let value = perms.remove(&key);
            let return_ = mutate(&mut value);
            perms.add(key, value);
            return_
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
        if (perms.contains(&key)) {
            let value = perms.remove(&key);
            mutate(&mut value);
            perms.add(key, value);
        } else {
            perms.add(key, default);
        }
    }

    /// Authorizes `permissioned` with a given capacity and increment the existing capacity if present.
    ///
    /// Consumption using `check_permission_consume` will deduct the capacity.
    public(package) fun authorize_increase<PermKey: copy + drop + store>(
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
    public(package) fun authorize_unlimited<PermKey: copy + drop + store>(
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

    /// Grant an unlimited permission to a permissioned signer **without** master signer's approvoal.
    public(package) fun grant_unlimited_with_permissioned_signer<PermKey: copy + drop + store>(
        permissioned: &signer,
        perm: PermKey
    ) acquires PermissionStorage {
        if(!is_permissioned_signer(permissioned)) {
            return;
        };
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
    public(package) fun increase_limit<PermKey: copy + drop + store>(
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

    public(package) fun check_permission_exists<PermKey: copy + drop + store>(
        s: &signer, perm: PermKey
    ): bool acquires PermissionStorage {
        // 0 capacity permissions will be treated as non-existant.
        check_permission_capacity_above(s, 1, perm)
    }

    public(package) fun check_permission_capacity_above<PermKey: copy + drop + store>(
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

    public(package) fun check_permission_consume<PermKey: copy + drop + store>(
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

    public(package) fun capacity<PermKey: copy + drop + store>(
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

    public(package) fun revoke_permission<PermKey: copy + drop + store>(
        permissioned: &signer, perm: PermKey
    ) acquires PermissionStorage {
        if (!is_permissioned_signer(permissioned)) {
            // Master signer has no permissions associated with it.
            return
        };
        let addr = permission_address(permissioned);
        if (!exists<PermissionStorage>(addr)) { return };
        let perm_storage = &mut PermissionStorage[addr].perms;
        let key = copyable_any::pack(perm);
        if (perm_storage.contains(&key)) {
            perm_storage.remove(&key);
        }
    }

    /// Unused function. Keeping it for compatibility purpose.
    public fun address_of(_s: &signer): address {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Unused function. Keeping it for compatibility purpose.
    public fun borrow_address(_s: &signer): &address {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    // =====================================================================================================
    // Native Functions
    ///
    /// Check whether this is a permissioned signer.
    native fun is_permissioned_signer_impl(s: &signer): bool;
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
        let velor_framework = create_signer(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

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

    #[test_only]
    use velor_std::bcs;

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x1C5, location = velor_std::bcs)]
    fun signer_serialization(
        creator: &signer
    ) acquires PermissionStorage {
        let velor_framework = create_signer(@0x1);
        timestamp::set_time_has_started_for_testing(&velor_framework);

        let handle = create_permissioned_handle(creator);
        let perm_signer = signer_from_permissioned_handle(&handle);

        assert!(bcs::to_bytes(creator) == bcs::to_bytes(&signer::address_of(creator)), 1);
        bcs::to_bytes(&perm_signer);

        destroy_permissioned_handle(handle);
    }
}
