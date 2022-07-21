/// A resource account is used to manage resources independent of an account managed by a user.
/// This contains several utilities to make using resource accounts more effective.
///
/// A dev wishing to use resource accounts for a liquidity pool, would likely do the following:
/// 1. Create a new account using `Resourceaccount::create_resource_account`. This creates the
/// account, stores the `signer_cap` within a `Resourceaccount::Container`, and rotates the key to
/// the current accounts authentication key or a provided authentication key.
/// 2. Define the LiquidityPool module's address to be the same as the resource account.
/// 3. Construct a ModuleBundle payload for the resource account using the authentication key used
/// in step 1.
/// 4. In the LiquidityPool module's `init_module` function, call `retrieve_resource_account_cap`
/// which will retrive the `signer_cap` and rotate the resource account's authentication key to
/// `0x0`, effectively locking it off.
/// 5. When adding a new coin, the liquidity pool will load the capability and hence the signer to
/// register and store new LiquidityCoin resources.
///
/// Code snippets to help:
/// ```
/// fun init_module(source: &signer) {
///   let dev_address = @DEV_ADDR;
///   let signer_cap = retrieve_resource_account_cap(&source, dev_address);
///   let lp_signer = create_signer_with_capability(&signer_cap);
///   let lp = LiquidityPoolInfo { signer_cap: signer_cap, ... };
///   move_to(&lp_signer, lp);
/// }
/// ```
///
/// Later on during a coin registration:
/// ```
/// public fun add_coin<X, Y>(lp: &LP, x: Coin<x>, y: Coin<y>) {
///     if(!exists<LiquidityCoin<X, Y>(LP::Address(lp), LiquidityCoin<X, Y>)) {
///         let mint, burn = Coin::initialize<LiquidityCoin<X, Y>>(...);
///         move_to(&create_signer_with_capability(&lp.cap), LiquidityCoin<X, Y>{ mint, burn });
///     }
///     ...
/// }
/// ```
module aptos_framework::resource_account {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_framework::account;
    use aptos_std::simple_map::{Self, SimpleMap};

    const ECONTAINER_NOT_PUBLISHED: u64 = 0;

    struct Container has key {
        store: SimpleMap<address, account::SignerCapability>,
    }

    /// Creates a new resource account and rotates the authentication key to either
    /// the optional auth key if it is non-empty (though auth keys are 32-bytes)
    /// or the source accounts current auth key.
    public entry fun create_resource_account(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
    ) acquires Container {
        let (resource, resource_signer_cap) = account::create_resource_account(origin, seed);

        let origin_addr = signer::address_of(origin);
        if (!exists<Container>(origin_addr)) {
            move_to(origin, Container { store: simple_map::create() })
        };

        let container = borrow_global_mut<Container>(origin_addr);
        let resource_addr = signer::address_of(&resource);
        simple_map::add(&mut container.store, resource_addr, resource_signer_cap);

        let auth_key = if (vector::is_empty(&optional_auth_key)) {
            account::get_authentication_key(origin_addr)
        } else {
            optional_auth_key
        };
        account::rotate_authentication_key_internal(&resource, auth_key);
    }

    /// When called by the resource account, it will retrieve the capability associated with that
    /// account and rotate the account's auth key to 0x0 making the account inaccessible without
    /// the SignerCapability.
    public fun retrieve_resource_account_cap(
        resource: &signer,
        source_addr: address,
    ): account::SignerCapability acquires Container {
        assert!(exists<Container>(source_addr), error::not_found(ECONTAINER_NOT_PUBLISHED));

        let resource_addr = signer::address_of(resource);
        let (resource_signer_cap, empty_container) = {
            let container = borrow_global_mut<Container>(source_addr);
            let (_resource_addr, signer_cap) = simple_map::remove(&mut container.store, &resource_addr);
            (signer_cap, simple_map::length(&container.store) == 0)
        };

        if (empty_container) {
            let container = move_from(source_addr);
            let Container { store: store } = container;
            simple_map::destroy_empty(store);
        };

        let zero_auth_key = x"0000000000000000000000000000000000000000000000000000000000000000";
        let resource = account::create_signer_with_capability(&resource_signer_cap);
        account::rotate_authentication_key_internal(&resource, zero_auth_key);
        resource_signer_cap
    }

    #[test(user = @0x1111)]
    public entry fun end_to_end(user: signer) acquires Container {
        use std::bcs;
        use std::hash;

        let user_addr = signer::address_of(&user);
        account::create_account(user_addr);

        let seed = x"01";
        let bytes = bcs::to_bytes(&user_addr);
        vector::append(&mut bytes, copy seed);
        let resource_addr = account::create_address_for_test(hash::sha3_256(bytes));

        create_resource_account(&user, seed, vector::empty());
        let container = borrow_global<Container>(user_addr);
        let resource_cap = simple_map::borrow(&container.store, &resource_addr);

        let resource = account::create_signer_with_capability(resource_cap);
        let _resource_cap = retrieve_resource_account_cap(&resource, user_addr);
    }
}
