/// A resource account is used to manage resources independent of an account managed by a user.
/// This contains several utilities to make using resource accounts more effective.
///
/// A dev wishing to use resource accounts for a liquidity pool, would likely do the following:
/// 1. Create a new account using `ResourceAccount::create_resource_account`. This creates the
/// account, stores the `signer_cap` within a `ResourceAccount::Container`, and rotates the key to
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
module AptosFramework::ResourceAccount {
    use Std::Errors;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::Account;
    use AptosFramework::SimpleMap::{Self, SimpleMap};

    const ECONTAINER_NOT_PUBLISHED: u64 = 0;

    struct Container has key {
        store: SimpleMap<address, Account::SignerCapability>,
    }

    /// Creates a new resource account and rotates the authentication key to either
    /// the optional auth key if it is non-empty (though auth keys are 32-bytes)
    /// or the source accounts current auth key.
    public(script) fun create_resource_account(
        origin: &signer,
        optional_auth_key: vector<u8>,
    ) acquires Container {
        let (resource, resource_signer_cap) = Account::create_resource_account(origin);

        let origin_addr = Signer::address_of(origin);
        if (!exists<Container>(origin_addr)) {
            move_to(origin, Container { store: SimpleMap::create() })
        };

        let container = borrow_global_mut<Container>(origin_addr);
        let resource_addr = Signer::address_of(&resource);
        SimpleMap::add(&mut container.store, resource_addr, resource_signer_cap);

        let auth_key = if (Vector::is_empty(&optional_auth_key)) {
            Account::get_authentication_key(origin_addr)
        } else {
            optional_auth_key
        };
        Account::rotate_authentication_key_internal(&resource, auth_key);
    }

    /// When called by the resource account, it will retrieve the capability associated with that
    /// account and rotate the account's auth key to 0x0 making the account inaccessible without
    /// the SignerCapability.
    public fun retrieve_resource_account_cap(
        resource: &signer,
        source_addr: address,
    ): Account::SignerCapability acquires Container {
        assert!(exists<Container>(source_addr), Errors::not_published(ECONTAINER_NOT_PUBLISHED));

        let resource_addr = Signer::address_of(resource);
        let (resource_signer_cap, empty_container) = {
            let container = borrow_global_mut<Container>(source_addr);
            let (_resource_addr, signer_cap) = SimpleMap::remove(&mut container.store, &resource_addr);
            (signer_cap, SimpleMap::length(&container.store) == 0)
        };

        if (empty_container) {
            let container = move_from(source_addr);
            let Container { store: store } = container;
            SimpleMap::destroy_empty(store);
        };

        let zero_auth_key = x"0000000000000000000000000000000000000000000000000000000000000000";
        let resource = Account::create_signer_with_capability(&resource_signer_cap);
        Account::rotate_authentication_key_internal(&resource, zero_auth_key);
        resource_signer_cap
    }

    #[test(user = @0x1111)]
    public(script) fun end_to_end(user: signer) acquires Container {
        use Std::BCS;
        use Std::GUID;
        use Std::Hash;

        let user_addr = Signer::address_of(&user);
        Account::create_account(user_addr);

        let next_creation_num = GUID::get_next_creation_num(user_addr);
        let id = GUID::create_id(user_addr, next_creation_num);
        let bytes = BCS::to_bytes(&id);
        let resource_addr = Account::create_address_for_test(Hash::sha3_256(bytes));

        create_resource_account(&user, Vector::empty());
        let container = borrow_global<Container>(user_addr);
        let resource_cap = SimpleMap::borrow(&container.store, &resource_addr);

        let resource = Account::create_signer_with_capability(resource_cap);
        let _resource_cap = retrieve_resource_account_cap(&resource, user_addr);
    }
}
