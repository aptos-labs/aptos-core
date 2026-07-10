/// The permissioned signer feature has been removed. This module remains as a shell for
/// upgrade compatibility: all type definitions are unchanged and every public function
/// aborts with `EPERMISSION_SIGNER_DISABLED`, except `is_permissioned_signer` which
/// returns `false`.
module aptos_framework::permissioned_signer {
    use std::error;
    use aptos_std::copyable_any::Any;
    use aptos_framework::big_ordered_map::BigOrderedMap;

    /// Permissioned signer feature is not activated.
    const EPERMISSION_SIGNER_DISABLED: u64 = 9;

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

    #[deprecated]
    public fun create_permissioned_handle(_master: &signer): PermissionedHandle {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    #[deprecated]
    public fun destroy_permissioned_handle(_p: PermissionedHandle) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    #[deprecated]
    public fun signer_from_permissioned_handle(_p: &PermissionedHandle): signer {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    #[deprecated]
    /// Permissioned signers no longer exist, so this always returns false.
    public fun is_permissioned_signer(_s: &signer): bool {
        false
    }

    #[deprecated]
    public fun grant_revoke_permission(_master: &signer, _permissioned: &signer) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    #[deprecated]
    public entry fun revoke_permission_storage_address(
        _s: &signer, _permissions_storage_addr: address
    ) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    #[deprecated]
    public entry fun revoke_all_handles(_s: &signer) {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    #[deprecated]
    public fun address_of(_s: &signer): address {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }

    #[deprecated]
    public fun borrow_address(_s: &signer): &address {
        abort error::permission_denied(EPERMISSION_SIGNER_DISABLED)
    }
}
