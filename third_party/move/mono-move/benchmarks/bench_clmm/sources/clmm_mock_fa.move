/// Test tokens for the pool, as real fungible assets.
///
/// Real assets are the point: every transfer touches a resource group on an
/// object account and computes a derived address, which is the storage traffic
/// a swap benchmark is meant to include.
module bench::clmm_mock_fa {
    use std::option;
    use std::signer;
    use std::string;
    use aptos_framework::fungible_asset::{Self, BurnRef, Metadata, MintRef, TransferRef};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;

    /// Caller does not hold the refs for this asset.
    const ENOT_ASSET_OWNER: u64 = 1;

    struct AssetRefs has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
    }

    /// Create an asset owned by `admin`, seeded by its symbol.
    ///
    /// The symbol doubles as the object seed, so the address is derivable from
    /// `(admin, symbol)` and a benchmark script needs no return value plumbing.
    public fun create_asset(
        admin: &signer, symbol: vector<u8>, decimals: u8
    ): Object<Metadata> {
        let constructor_ref = &object::create_named_object(admin, symbol);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            string::utf8(symbol),
            string::utf8(symbol),
            decimals,
            string::utf8(b""),
            string::utf8(b"")
        );
        let asset_signer = &object::generate_signer(constructor_ref);
        move_to(
            asset_signer,
            AssetRefs {
                mint_ref: fungible_asset::generate_mint_ref(constructor_ref),
                burn_ref: fungible_asset::generate_burn_ref(constructor_ref),
                transfer_ref: fungible_asset::generate_transfer_ref(constructor_ref),
            }
        );
        object::object_from_constructor_ref<Metadata>(constructor_ref)
    }

    public entry fun create_asset_entry(
        admin: &signer, symbol: vector<u8>, decimals: u8
    ) {
        create_asset(admin, symbol, decimals);
    }

    public entry fun mint(
        admin: &signer, asset: Object<Metadata>, to: address, amount: u64
    ) acquires AssetRefs {
        assert!(object::is_owner(asset, signer::address_of(admin)), ENOT_ASSET_OWNER);
        faucet(asset, to, amount);
    }

    /// Unpermissioned mint. A harness drives thousands of independent accounts
    /// that each need funding, and routing every one through the admin would
    /// serialize them on the admin's sequence number.
    public fun faucet(
        asset: Object<Metadata>, to: address, amount: u64
    ) acquires AssetRefs {
        let refs = borrow_global<AssetRefs>(object::object_address(&asset));
        primary_fungible_store::mint(&refs.mint_ref, to, amount);
    }

    #[view]
    public fun asset_address(creator: address, symbol: vector<u8>): address {
        object::create_object_address(&creator, symbol)
    }

    #[view]
    public fun metadata(creator: address, symbol: vector<u8>): Object<Metadata> {
        object::address_to_object<Metadata>(asset_address(creator, symbol))
    }

    #[view]
    public fun balance(owner: address, asset: Object<Metadata>): u64 {
        primary_fungible_store::balance(owner, asset)
    }

    //
    // Tests.
    //

    #[test(admin = @bench)]
    fun test_create_and_mint(admin: &signer) acquires AssetRefs {
        let asset = create_asset(admin, b"TKA", 8);
        assert!(object::object_address(&asset) == asset_address(@bench, b"TKA"), 0);
        assert!(metadata(@bench, b"TKA") == asset, 0);
        assert!(balance(@bench, asset) == 0, 0);

        mint(admin, asset, @bench, 1000000);
        assert!(balance(@bench, asset) == 1000000, 0);
        mint(admin, asset, @0xB1, 500);
        assert!(balance(@0xB1, asset) == 500, 0);
        assert!(balance(@bench, asset) == 1000000, 0);
    }

    #[test(admin = @bench)]
    fun test_two_assets_are_distinct(admin: &signer) acquires AssetRefs {
        let a = create_asset(admin, b"TKA", 8);
        let b = create_asset(admin, b"TKB", 6);
        assert!(a != b, 0);
        mint(admin, a, @bench, 100);
        assert!(balance(@bench, a) == 100, 0);
        assert!(balance(@bench, b) == 0, 0);
    }

    #[test(admin = @bench, other = @0xB1)]
    #[expected_failure(abort_code = ENOT_ASSET_OWNER, location = Self)]
    fun test_only_the_owner_may_mint(admin: &signer, other: &signer) acquires AssetRefs {
        let asset = create_asset(admin, b"TKA", 8);
        mint(other, asset, @0xB1, 1);
    }
}
