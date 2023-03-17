module fungible_asset::fungible_source {
    use aptos_framework::object::{object_address, Object, ConstructorRef, address_to_object};
    use std::option::Option;
    use std::option;
    use std::error;
    use fungible_asset::fungible_asset::{set_frozen_flag, deposit, FungibleAsset};
    use aptos_framework::object;
    use std::signer;
    use fungible_asset::fungible_asset;

    /// The fungible asset supply exists or does not exist for this asset object.
    const EFUNGIBLE_SOURCE: u64 = 1;
    /// Amount cannot be zero.
    const EZERO_AMOUNT: u64 = 2;
    /// The current_supply of token as fungible assets is not zero.
    const ECURRENT_SUPPLY_NON_ZERO: u64 = 3;
    /// Mint capability exists or does not exist.
    const EMINT_CAP: u64 = 4;
    /// Freeze capability exists does not exist.
    const EFREEZE_CAP: u64 = 5;
    /// Burn capability exists or does not exist.
    const EBURN_CAP: u64 = 6;
    /// Current supply overflow
    const ECURRENT_SUPPLY_OVERFLOW: u64 = 7;
    /// Current supply underflow
    const ECURRENT_SUPPLY_UNDERFLOW: u64 = 8;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct FungibleSource has key {
        current_supply: u64,
        maximum_supply: Option<u64>,
        /// Number of decimals used to get its user representation.
        /// For example, if `decimals` equals `2`, a balance of `505` coins should
        /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
        decimals: u8,
    }

    /// Capabilities
    struct MintCap has store {
        asset_addr: address
    }

    struct FreezeCap has store {
        asset_addr: address
    }

    struct BurnCap has store {
        asset_addr: address
    }

    public fun init_fungible_source(
        constructor_ref: &ConstructorRef,
        maximum_supply: u64,
        decimals: u8,
    ): (MintCap, FreezeCap, BurnCap) {
        let asset_object_signer = object::generate_signer(constructor_ref);
        let converted_maximum = if (maximum_supply == 0) {
            option::none()
        } else {
            option::some(maximum_supply)
        };
        move_to(&asset_object_signer,
            FungibleSource {
                current_supply: 0,
                maximum_supply: converted_maximum,
                decimals,
            }
        );
        let asset_addr = signer::address_of(&asset_object_signer);
        (MintCap { asset_addr }, FreezeCap { asset_addr }, BurnCap { asset_addr })
    }


    public fun get_current_supply<T: key>(asset: &Object<T>): u64 acquires FungibleSource {
        let asset_addr = verify(asset);
        borrow_fungible_source(asset_addr).current_supply
    }

    public fun get_maximum_supply<T: key>(asset: &Object<T>): Option<u64> acquires FungibleSource {
        let asset_addr = verify(asset);
        borrow_fungible_source(asset_addr).maximum_supply
    }

    public fun get_decimals<T: key>(asset: &Object<T>): u8 acquires FungibleSource {
        let asset_addr = verify(asset);
        borrow_fungible_source(asset_addr).decimals
    }

    /// Mint the `amount` of coin with MintCap.
    public fun mint(
        cap: &MintCap,
        amount: u64,
        to: address
    ) acquires FungibleSource {
        // This ensures amount > 0;
        increase_supply(cap, amount);
        let fa = fungible_asset::mint(cap.asset_addr, amount);
        deposit(fa, to);
    }

    /// Freeze the fungible asset account of `fungible_asset_owner` with FreezeCap.
    public fun freeze_(
        cap: &FreezeCap,
        fungible_asset_owner: address,
    ) {
        set_frozen(cap, fungible_asset_owner, true);
    }

    /// Unfreeze the fungible asset account of `fungible_asset_owner` with FreezeCap.
    public fun unfreeze(
        cap: &FreezeCap,
        fungible_asset_owner: address,
    ) {
        set_frozen(cap, fungible_asset_owner, false);
    }

    fun set_frozen(
        cap: &FreezeCap,
        fungible_asset_owner: address,
        frozen: bool
    ) {
        set_frozen_flag(fungible_asset_owner, cap.asset_addr, frozen);
    }

    /// Burn the `amount` of coin with MintCap.
    public fun burn(
        cap: &BurnCap,
        amount: u64,
        from_account: address
    ) acquires FungibleSource {
        decrease_supply(cap, amount);
        let fungible_asset_to_burn = fungible_asset::withdraw(from_account, cap.asset_addr, amount);
        fungible_asset::burn(fungible_asset_to_burn);
    }

    public fun withdraw<T: key>(
        fungible_asset_owner: &signer,
        asset: &Object<T>,
        amount: u64
    ): FungibleAsset {
        // Verify the passed-in object is a fungible source.
        let asset_addr = verify(asset);
        let account_address = signer::address_of(fungible_asset_owner);
        fungible_asset::withdraw(account_address, asset_addr, amount)
    }

    // Moves balances around and not the underlying object.
    public fun transfer<T: key>(
        fungible_asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        to: address
    ) {
        let fa = withdraw(fungible_asset_owner, asset, amount);
        deposit(fa, to);
    }

    /// Increase the supply of a fungible asset by minting.
    public fun increase_supply(cap: &MintCap, amount: u64) acquires FungibleSource {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        let fungible_source = borrow_fungible_source_mut(cap.asset_addr);
        if (option::is_some(&fungible_source.maximum_supply)) {
            let max = *option::borrow(&fungible_source.maximum_supply);
            assert!(max - fungible_source.current_supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_OVERFLOW))
        };
        fungible_source.current_supply = fungible_source.current_supply + amount;
    }

    /// Increase the supply of a fungible asset by burning.
    public fun decrease_supply(cap: &BurnCap, amount: u64) acquires FungibleSource {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        let fungible_source = borrow_fungible_source_mut(cap.asset_addr);
        assert!(fungible_source.current_supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_UNDERFLOW));
        fungible_source.current_supply = fungible_source.current_supply - amount;
    }

    public fun destroy_mint_cap(cap: MintCap) {
        let MintCap { asset_addr: _ } = cap;
    }

    public fun destroy_freeze_cap(cap: FreezeCap) {
        let FreezeCap { asset_addr: _ } = cap;
    }

    public fun destroy_burn_cap(cap: BurnCap) {
        let BurnCap { asset_addr: _ } = cap;
    }

    /// Borrow a `&FungibleSource` from an asset.
    inline fun borrow_fungible_source(asset_addr: address): &FungibleSource acquires FungibleSource {
        borrow_global<FungibleSource>(asset_addr)
    }

    /// Borrow a `&mut FungibleSource` from an asset.
    inline fun borrow_fungible_source_mut(asset_addr: address): &mut FungibleSource acquires FungibleSource {
        borrow_global_mut<FungibleSource>(asset_addr)
    }

    public inline fun verify<T: key>(asset: &Object<T>): address {
        let addr = object_address(asset);
        address_to_object<FungibleSource>(addr);
        addr
    }

    #[test_only]
    use fungible_asset::fungible_asset::{create_test_token, is_frozen};
    #[test_only]
    use std::signer::address_of;

    #[test_only]
    public fun destroy_caps(mint_cap: MintCap, freeze_cap: FreezeCap, burn_cap: BurnCap) {
        destroy_mint_cap(mint_cap);
        destroy_freeze_cap(freeze_cap);
        destroy_burn_cap(burn_cap);
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flows(creator: &signer) acquires FungibleSource {
        let (creator_ref, asset) = create_test_token(creator);
        let (mint_cap, freeze_cap, burn_cap) = init_fungible_source(
            &creator_ref,
            100 /* max supply */,
            0
        );
        let creator_address = address_of(creator);
        assert!(get_current_supply(&asset) == 0, 1);
        assert!(get_maximum_supply(&asset) == option::some(100), 1);
        mint(&mint_cap, 100, creator_address);
        assert!(get_current_supply(&asset) == 100, 2);
        freeze_(&freeze_cap, creator_address);
        assert!(is_frozen(creator_address, &asset), 3);
        unfreeze(&freeze_cap, creator_address);
        assert!(!is_frozen(creator_address, &asset), 4);
        burn(&burn_cap, 90, creator_address);
        assert!(get_current_supply(&asset) == 10, 5);
        destroy_caps(mint_cap, freeze_cap, burn_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10007, location = Self)]
    fun test_supply_overflow(creator: &signer) acquires FungibleSource {
        let (creator_ref, _asset) = create_test_token(creator);
        let (mint_cap, freeze_cap, burn_cap) = init_fungible_source(
            &creator_ref,
            100 /* max supply */,
            0
        );
        let creator_address = address_of(creator);
        mint(&mint_cap, 101, creator_address);
        destroy_caps(mint_cap, freeze_cap, burn_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10008, location = Self)]
    fun test_supply_underflow(creator: &signer) acquires FungibleSource {
        let (creator_ref, _asset) = create_test_token(creator);
        let (mint_cap, freeze_cap, burn_cap) = init_fungible_source(
            &creator_ref,
            100 /* max supply */,
            0
        );
        let creator_address = address_of(creator);
        burn(&burn_cap, 1, creator_address);
        destroy_caps(mint_cap, freeze_cap, burn_cap);
    }
}
