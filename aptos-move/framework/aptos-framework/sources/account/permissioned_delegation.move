/// The permissioned signer feature has been removed. This module remains as a shell for
/// upgrade compatibility: type definitions are unchanged and the public functions abort
/// with `EPERMISSIONED_SIGNER_REMOVED`.
module aptos_framework::permissioned_delegation {
    use std::error;
    use std::option::Option;
    use aptos_std::ed25519::UnvalidatedPublicKey;
    use aptos_std::big_ordered_map::BigOrderedMap;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::permissioned_signer::StorablePermissionedHandle;
    use aptos_framework::rate_limiter::RateLimiter;

    /// The permissioned signer feature has been removed.
    const EPERMISSIONED_SIGNER_REMOVED: u64 = 7;

    #[deprecated]
    enum AccountDelegation has store {
        V1 { handle: StorablePermissionedHandle, rate_limiter: Option<RateLimiter> }
    }

    #[deprecated]
    enum DelegationKey has copy, store, drop {
        Ed25519PublicKey(UnvalidatedPublicKey)
    }

    #[deprecated]
    struct RegisteredDelegations has key {
        delegations: BigOrderedMap<DelegationKey, AccountDelegation>
    }

    #[deprecated]
    public fun gen_ed25519_key(key: UnvalidatedPublicKey): DelegationKey {
        DelegationKey::Ed25519PublicKey(key)
    }

    #[deprecated]
    public fun add_permissioned_handle(
        _master: &signer,
        _key: DelegationKey,
        _rate_limiter: Option<RateLimiter>,
        _expiration_time: u64,
    ): signer {
        abort error::unavailable(EPERMISSIONED_SIGNER_REMOVED)
    }

    #[deprecated]
    public fun remove_permissioned_handle(
        _master: &signer,
        _key: DelegationKey,
    ) {
        abort error::unavailable(EPERMISSIONED_SIGNER_REMOVED)
    }

    #[deprecated]
    public fun permissioned_signer_by_key(
        _master: &signer,
        _key: DelegationKey,
    ): signer {
        abort error::unavailable(EPERMISSIONED_SIGNER_REMOVED)
    }

    #[deprecated]
    public fun handle_address_by_key(_master: address, _key: DelegationKey): address {
        abort error::unavailable(EPERMISSIONED_SIGNER_REMOVED)
    }

    #[deprecated]
    public fun authenticate(
        _account: signer,
        _abstraction_auth_data: AbstractionAuthData
    ): signer {
        abort error::unavailable(EPERMISSIONED_SIGNER_REMOVED)
    }
}
