/// Deprecated. This module was built on the permissioned signer feature (AIP-103), which was
/// never enabled and has been removed. The structs are retained for upgrade compatibility and
/// the functions are neutralized to abort.
module aptos_framework::permissioned_delegation {
    use std::error;
    use std::option::Option;
    use aptos_std::ed25519::UnvalidatedPublicKey;
    use aptos_std::big_ordered_map::BigOrderedMap;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::permissioned_signer::StorablePermissionedHandle;
    use aptos_framework::rate_limiter::{Self, RateLimiter};

    /// Permissioned delegation is deprecated and disabled.
    const EDELEGATION_DISABLED: u64 = 1;

    #[deprecated]
    enum AccountDelegation has store {
        V1 { handle: StorablePermissionedHandle, rate_limiter: Option<rate_limiter::RateLimiter> }
    }

    #[deprecated]
    enum DelegationKey has copy, store, drop {
        Ed25519PublicKey(UnvalidatedPublicKey)
    }

    public fun gen_ed25519_key(key: UnvalidatedPublicKey): DelegationKey {
        DelegationKey::Ed25519PublicKey(key)
    }

    #[deprecated]
    struct RegisteredDelegations has key {
        delegations: BigOrderedMap<DelegationKey, AccountDelegation>
    }

    /// Deprecated. Permissioned delegation was never enabled. Aborts.
    public fun add_permissioned_handle(
        _master: &signer,
        _key: DelegationKey,
        _rate_limiter: Option<RateLimiter>,
        _expiration_time: u64,
    ): signer {
        abort error::permission_denied(EDELEGATION_DISABLED)
    }

    /// Deprecated. Permissioned delegation was never enabled. Aborts.
    public fun remove_permissioned_handle(_master: &signer, _key: DelegationKey) {
        abort error::permission_denied(EDELEGATION_DISABLED)
    }

    /// Deprecated. Permissioned delegation was never enabled. Aborts.
    public fun permissioned_signer_by_key(_master: &signer, _key: DelegationKey): signer {
        abort error::permission_denied(EDELEGATION_DISABLED)
    }

    /// Deprecated. Permissioned delegation was never enabled. Aborts.
    public fun handle_address_by_key(_master: address, _key: DelegationKey): address {
        abort error::permission_denied(EDELEGATION_DISABLED)
    }

    /// Authorization function for account abstraction. Deprecated and disabled; aborts.
    public fun authenticate(
        _account: signer,
        _abstraction_auth_data: AbstractionAuthData
    ): signer {
        abort error::permission_denied(EDELEGATION_DISABLED)
    }

    spec module {
        pragma verify = false;
    }
}
