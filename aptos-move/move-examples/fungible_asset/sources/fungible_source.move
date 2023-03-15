module fungible_asset::fungible_source {

    use aptos_framework::object::{object_address, Object, ExtendRef, address_from_extend_ref};
    use std::option::Option;
    use std::option;
    use std::error;
    use fungible_asset::fungible_asset::{mint, set_frozen_flag, withdraw_internal, burn, deposit};
    use aptos_framework::object;
    use std::signer;
    #[test_only]
    use fungible_asset::fungible_asset::{create_test_token, is_frozen};
    #[test_only]
    use aptos_framework::object::generate_extend_ref;
    #[test_only]
    use std::signer::address_of;

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
        maximum_supply: Option<u64>
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
        extend_ref: &ExtendRef,
        maximum_supply: u64
    ): (MintCap, FreezeCap, BurnCap) {
        assert_fungible_source_not_exists(address_from_extend_ref(extend_ref));
        let asset_object_signer = object::generate_signer_for_extending(extend_ref);
        let converted_maximum = if (maximum_supply == 0) {
            option::none()
        } else {
            option::some(maximum_supply)
        };
        move_to(&asset_object_signer,
            FungibleSource {
                current_supply: 0,
                maximum_supply: converted_maximum
            }
        );
        let asset_addr = signer::address_of(&asset_object_signer);
        (MintCap { asset_addr }, FreezeCap { asset_addr }, BurnCap { asset_addr })
    }


    public fun get_current_supply<T: key>(asset: &Object<T>): u64 acquires FungibleSource {
        borrow_fungible_source(asset).current_supply
    }

    public fun get_maximum_supply<T: key>(asset: &Object<T>): Option<u64> acquires FungibleSource {
        borrow_fungible_source(asset).maximum_supply
    }

    public fun assert_fungible_source_exists(asset_address: address) {
        assert!(fungible_source_exists(asset_address), error::not_found(EFUNGIBLE_SOURCE));
    }

    public fun assert_fungible_source_not_exists(asset_address: address) {
        assert!(!fungible_source_exists(asset_address), error::already_exists(EFUNGIBLE_SOURCE));
    }

    public fun fungible_source_exists(asset_address: address): bool {
        exists<FungibleSource>(asset_address)
    }

    /// Mint the `amount` of coin with MintCap.
    public fun mint_with_cap<T: key>(
        cap: &MintCap,
        asset: &Object<T>,
        amount: u64,
        to: address
    ) acquires FungibleSource {
        // This ensures amount > 0;
        increase_supply(cap, asset, amount);
        let fa = mint(object_address(asset), amount);
        deposit(fa, to);
    }

    /// Freeze the fungible asset account of `fungible_asset_owner` with FreezeCap.
    public fun freeze_with_cap<T: key>(
        cap: &FreezeCap,
        fungible_asset_owner: address,
        asset: &Object<T>
    ) {
        set_frozen_with_cap(cap, fungible_asset_owner, asset, true);
    }

    /// Unfreeze the fungible asset account of `fungible_asset_owner` with FreezeCap.
    public fun unfreeze_with_cap<T: key>(
        cap: &FreezeCap,
        fungible_asset_owner: address,
        asset: &Object<T>
    ) {
        set_frozen_with_cap(cap, fungible_asset_owner, asset, false);
    }

    fun set_frozen_with_cap<T: key>(
        cap: &FreezeCap,
        fungible_asset_owner: address,
        asset: &Object<T>,
        frozen: bool
    ) {
        assert_freeze_cap_and_asset_match(cap, asset);
        set_frozen_flag(fungible_asset_owner, asset, frozen);
    }

    /// Burn the `amount` of coin with MintCap.
    public fun burn_with_cap<T: key>(
        cap: &BurnCap,
        asset: &Object<T>,
        amount: u64,
        from_account: address
    ) acquires FungibleSource {
        // This ensures amount > 0;
        decrease_supply(cap, asset, amount);
        let fungible_asset_to_burn = withdraw_internal(from_account, asset, amount);
        burn(fungible_asset_to_burn);
    }

    /// Increase the supply of a fungible asset by minting.
    public fun increase_supply<T: key>(cap: &MintCap, asset: &Object<T>, amount: u64) acquires FungibleSource {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        assert_mint_cap_and_asset_match(cap, asset);
        let fungible_source = borrow_fungible_source_mut(asset);
        if (option::is_some(&fungible_source.maximum_supply)) {
            let max = *option::borrow(&fungible_source.maximum_supply);
            assert!(max - fungible_source.current_supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_OVERFLOW))
        };
        fungible_source.current_supply = fungible_source.current_supply + amount;
    }

    /// Increase the supply of a fungible asset by burning.
    public fun decrease_supply<T: key>(cap: &BurnCap, asset: &Object<T>, amount: u64) acquires FungibleSource {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        assert_burn_cap_and_asset_match(cap, asset);
        let fungible_source = borrow_fungible_source_mut(asset);
        assert!(fungible_source.current_supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_UNDERFLOW));
        fungible_source.current_supply = fungible_source.current_supply - amount;
    }

    public fun destory_mint_cap(cap: MintCap) {
        let MintCap { asset_addr: _ } = cap;
    }

    public fun destory_freeze_cap(cap: FreezeCap) {
        let FreezeCap { asset_addr: _ } = cap;
    }

    public fun destory_burn_cap(cap: BurnCap) {
        let BurnCap { asset_addr: _ } = cap;
    }

    inline fun assert_mint_cap_and_asset_match<T: key>(cap: &MintCap, asset: &Object<T>) {
        assert!(cap.asset_addr == object_address(asset), error::invalid_argument(EMINT_CAP));
    }

    inline fun assert_freeze_cap_and_asset_match<T: key>(cap: &FreezeCap, asset: &Object<T>) {
        assert!(cap.asset_addr == object_address(asset), error::invalid_argument(EFREEZE_CAP));
    }

    inline fun assert_burn_cap_and_asset_match<T: key>(cap: &BurnCap, asset: &Object<T>) {
        assert!(cap.asset_addr == object_address(asset), error::invalid_argument(EBURN_CAP));
    }

    /// Borrow a `&FungibleSource` from an asset.
    inline fun borrow_fungible_source<T: key>(asset: &Object<T>): &FungibleSource acquires FungibleSource {
        let object_addr = verify(asset);
        borrow_global<FungibleSource>(object_addr)
    }

    /// Borrow a `&mut FungibleSource` from an asset.
    inline fun borrow_fungible_source_mut<T: key>(asset: &Object<T>): &mut FungibleSource acquires FungibleSource {
        let object_addr = verify(asset);
        borrow_global_mut<FungibleSource>(object_addr)
    }

    inline fun verify<T: key>(fungible_source: &Object<T>): address {
        let fungible_source_address = object::object_address(fungible_source);
        assert!(
            exists<FungibleSource>(fungible_source_address),
            error::not_found(EFUNGIBLE_SOURCE),
        );
        fungible_source_address
    }

    #[test_only]
    public fun destroy_caps(mint_cap: MintCap, freeze_cap: FreezeCap, burn_cap: BurnCap) {
        destory_mint_cap(mint_cap);
        destory_freeze_cap(freeze_cap);
        destory_burn_cap(burn_cap);
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flow_with_caps(creator: &signer) acquires FungibleSource {
        let (creator_ref, asset) = create_test_token(creator);
        let (mint_cap, freeze_cap, burn_cap) = init_fungible_source(
            &generate_extend_ref(&creator_ref),
            100 /* max supply */
        );
        let creator_address = address_of(creator);
        assert!(get_current_supply(&asset) == 0, 1);
        assert!(get_maximum_supply(&asset) == option::some(100), 1);
        mint_with_cap(&mint_cap, &asset, 100, creator_address);
        assert!(get_current_supply(&asset) == 100, 2);
        freeze_with_cap(&freeze_cap, creator_address, &asset);
        assert!(is_frozen(creator_address, &asset), 3);
        unfreeze_with_cap(&freeze_cap, creator_address, &asset);
        assert!(!is_frozen(creator_address, &asset), 4);
        burn_with_cap(&burn_cap, &asset, 90, creator_address);
        assert!(get_current_supply(&asset) == 10, 5);
        destroy_caps(mint_cap, freeze_cap, burn_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10007, location = Self)]
    fun test_supply_overflow(creator: &signer) acquires FungibleSource {
        let (creator_ref, asset) = create_test_token(creator);
        let (mint_cap, freeze_cap, burn_cap) = init_fungible_source(
            &generate_extend_ref(&creator_ref),
            100 /* max supply */
        );
        let creator_address = address_of(creator);
        mint_with_cap(&mint_cap, &asset, 101, creator_address);
        destroy_caps(mint_cap, freeze_cap, burn_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10008, location = Self)]
    fun test_supply_underflow(creator: &signer) acquires FungibleSource {
        let (creator_ref, asset) = create_test_token(creator);
        let (mint_cap, freeze_cap, burn_cap) = init_fungible_source(
            &generate_extend_ref(&creator_ref),
            100 /* max supply */
        );
        let creator_address = address_of(creator);
        burn_with_cap(&burn_cap, &asset, 1, creator_address);
        destroy_caps(mint_cap, freeze_cap, burn_cap);
    }
}
