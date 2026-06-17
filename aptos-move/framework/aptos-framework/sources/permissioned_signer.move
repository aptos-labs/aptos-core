/// Deprecated. The permissioned signer feature (AIP-103) was never enabled and has been removed.
///
/// The structs and enums are retained for upgrade compatibility and are marked `#[deprecated]`.
/// All functions are neutralized: signer creation and permission granting abort, permission
/// checks behave as if the caller is always a master signer (which holds every permission).
module aptos_framework::permissioned_signer {
    use std::error;
    use std::option::{Option, Self};
    use aptos_std::copyable_any::Any;
    use aptos_framework::big_ordered_map::BigOrderedMap;

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

    #[deprecated]
    /// If a permissioned signer has this permission, it would be able to revoke other granted
    /// permission handles in the same signer.
    struct RevokePermissionHandlePermission has copy, store, drop {}

    #[deprecated]
    /// Stores the list of granted permission handles for a given account.
    struct GrantedPermissionHandles has key {
        /// Each address refers to a `permissions_storage_addr` that stores the `PermissionStorage`.
        active_handles: vector<address>
    }

    #[deprecated]
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

    #[deprecated]
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

    #[deprecated]
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

    #[deprecated]
    /// Types of permission capacity stored on chain.
    enum StoredPermission has store, copy, drop {
        /// Unlimited capacity.
        Unlimited,
        /// Fixed capacity, will be deducted when permission is used.
        Capacity(u256),
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public fun create_permissioned_handle(_master: &signer): PermissionedHandle {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public fun destroy_permissioned_handle(_p: PermissionedHandle) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public fun signer_from_permissioned_handle(_p: &PermissionedHandle): signer {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled, so no signer is ever permissioned.
    public fun is_permissioned_signer(_s: &signer): bool {
        false
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public fun grant_revoke_permission(_master: &signer, _permissioned: &signer) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public entry fun revoke_permission_storage_address(
        _s: &signer, _permissions_storage_addr: address
    ) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public entry fun revoke_all_handles(_s: &signer) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public(package) fun create_storable_permissioned_handle(
        _master: &signer, _expiration_time: u64
    ): StorablePermissionedHandle {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public(package) fun destroy_storable_permissioned_handle(_p: StorablePermissionedHandle) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public(package) fun signer_from_storable_permissioned_handle(
        _p: &StorablePermissionedHandle
    ): signer {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public(package) fun permissions_storage_address(_p: &StorablePermissionedHandle): address {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Every signer is a master signer, so this is a no-op.
    public(package) fun assert_master_signer(_s: &signer) {}

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public(package) fun authorize_increase<PermKey: copy + drop + store>(
        _master: &signer, _permissioned: &signer, _capacity: u256, _perm: PermKey
    ) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled. Aborts.
    public(package) fun authorize_unlimited<PermKey: copy + drop + store>(
        _master: &signer, _permissioned: &signer, _perm: PermKey
    ) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Deprecated. Permissioned signers were never enabled, so this is a no-op.
    public(package) fun grant_unlimited_with_permissioned_signer<PermKey: copy + drop + store>(
        _permissioned: &signer, _perm: PermKey
    ) {}

    /// Deprecated. Permissioned signers were never enabled, so this is a no-op.
    public(package) fun increase_limit<PermKey: copy + drop + store>(
        _permissioned: &signer, _capacity: u256, _perm: PermKey
    ) {}

    /// Deprecated. Every signer is a master signer, which holds every permission.
    public(package) fun check_permission_exists<PermKey: copy + drop + store>(
        _s: &signer, _perm: PermKey
    ): bool {
        true
    }

    /// Deprecated. Every signer is a master signer, which holds every permission.
    public(package) fun check_permission_capacity_above<PermKey: copy + drop + store>(
        _s: &signer, _threshold: u256, _perm: PermKey
    ): bool {
        true
    }

    /// Deprecated. Every signer is a master signer, which holds every permission.
    public(package) fun check_permission_consume<PermKey: copy + drop + store>(
        _s: &signer, _threshold: u256, _perm: PermKey
    ): bool {
        true
    }

    /// Deprecated. Every signer is a master signer, which has unlimited capacity.
    public(package) fun capacity<PermKey: copy + drop + store>(
        _s: &signer, _perm: PermKey
    ): Option<u256> {
        option::some(U256_MAX)
    }

    /// Deprecated. Permissioned signers were never enabled, so this is a no-op.
    public(package) fun revoke_permission<PermKey: copy + drop + store>(
        _permissioned: &signer, _perm: PermKey
    ) {}

    /// Unused function. Keeping it for compatibility purpose.
    public fun address_of(_s: &signer): address {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    /// Unused function. Keeping it for compatibility purpose.
    public fun borrow_address(_s: &signer): &address {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }
}
