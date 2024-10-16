// benchmark is run without indexer, so we need to do bookeeping onchain.

module publisher_address::liquidity_pool_wrapper {
    use aptos_framework::fungible_asset::{Self, Metadata, MintRef};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use publisher_address::liquidity_pool::{Self, LiquidityPool};
    use std::option;
    use std::string;
    use std::signer;

    const EADDRESSES_NOT_FOUND: u64 = 1;
    const ENOT_PUBLISHER: u64 = 2;

    struct Addresses has key {
        pool: Object<LiquidityPool>,
        token_1_mint: MintRef,
        token_2_mint: MintRef,
        token_1: Object<Metadata>,
        token_2: Object<Metadata>,
    }

    public entry fun initialize_liquid_pair(publisher: &signer, is_stable: bool) {
        assert!(signer::address_of(publisher) == @publisher_address, ENOT_PUBLISHER);

        let (pool, token_1_mint, token_2_mint) = create_pool(publisher, is_stable);

        // Add liquidity to the pool
        add_liquidity(publisher, &token_1_mint, &token_2_mint, 100000000, 200000000, is_stable);
        if (!is_stable) {
            verify_reserves(pool, 100000000, 200000000);
        };
        let token_1 = fungible_asset::mint_ref_metadata(&token_1_mint);
        let token_2 = fungible_asset::mint_ref_metadata(&token_2_mint);
        move_to(
            publisher,
            Addresses {
                pool: pool,
                token_1_mint,
                token_2_mint,
                token_1,
                token_2,
            }
        )
    }

    public entry fun swap(
        user: &signer,
        publisher: &signer,
        amount_in: u64,
        from_1: bool,
    ) acquires Addresses {
        assert!(signer::address_of(publisher) == @publisher_address, ENOT_PUBLISHER);
        assert!(exists<Addresses>(@publisher_address), EADDRESSES_NOT_FOUND);
        let addresses = borrow_global<Addresses>(@publisher_address);

        let (from_token, from_mint) = if (from_1) {
            (addresses.token_1, &addresses.token_1_mint)
        } else {
            (addresses.token_2, &addresses.token_2_mint)
        };

        let tokens = if (primary_fungible_store::balance(signer::address_of(user), from_token) >= amount_in) {
            primary_fungible_store::withdraw(user, from_token, amount_in)
        } else {
            fungible_asset::mint(from_mint, amount_in)
        };
        let out = liquidity_pool::swap(publisher, addresses.pool, tokens);
        primary_fungible_store::deposit(signer::address_of(user), out);
    }

    fun verify_reserves(pool: Object<LiquidityPool>, expected_1: u64, expected_2: u64) {
        let (reserves_1, reserves_2) = liquidity_pool::pool_reserves(pool);
        assert!(reserves_1 == expected_1, 0);
        assert!(reserves_2 == expected_2, 0);
    }

    public fun add_liquidity(
        lp: &signer,
        mint_1_ref: &MintRef,
        mint_2_ref: &MintRef,
        amount_1: u64,
        amount_2: u64,
        is_stable: bool,
    ) {
        liquidity_pool::mint(
            lp,
            fungible_asset::mint(mint_1_ref, amount_1),
            fungible_asset::mint(mint_2_ref, amount_2),
            is_stable,
        );
    }

    public fun create_pool(publisher: &signer, is_stable: bool): (Object<LiquidityPool>, MintRef, MintRef) {
        let mint_1 = create_fungible_asset(publisher, b"test1");
        let mint_2 = create_fungible_asset(publisher, b"test2");
        let pool = liquidity_pool::create(
            publisher,
            fungible_asset::mint_ref_metadata(&mint_1),
            fungible_asset::mint_ref_metadata(&mint_2),
            is_stable,
        );
        (pool, mint_1, mint_2)
    }

    public fun create_fungible_asset(creator: &signer, name: vector<u8>): MintRef {
        let token_metadata = &object::create_named_object(creator, name);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            token_metadata,
            option::none(),
            string::utf8(name),
            string::utf8(name),
            8,
            string::utf8(b""),
            string::utf8(b""),
        );
        fungible_asset::generate_mint_ref(token_metadata)
    }

}
