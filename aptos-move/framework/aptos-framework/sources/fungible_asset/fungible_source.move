/// This module defines the extension called `FungibleSource` that any object must equip with to make it fungible.
module aptos_framework::fungible_source {
    use aptos_framework::object::{Self, Object, ConstructorRef};
    use std::option::Option;
    use std::option;
    use std::error;
    use std::string::String;

    friend aptos_framework::fungible_caps;
    friend aptos_framework::fungible_asset;

    /// Amount cannot be zero.
    const EZERO_AMOUNT: u64 = 1;
    /// Current supply overflow
    const ECURRENT_SUPPLY_OVERFLOW: u64 = 2;
    /// Current supply underflow
    const ECURRENT_SUPPLY_UNDERFLOW: u64 = 3;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Define the metadata required of an asset to be fungible.
    struct FungibleSource has key {
        /// Self-explanatory.
        current_supply: u64,
        /// The max supply limit where `option::none()` means no limit.
        maximum_supply: Option<u64>,
        /// Name of the fungible asset, i.e., "USDT".
        name: String,
        /// Symbol of the fungible asset, usually a shorter version of the name.
        /// For example, Singapore Dollar is SGD.
        symbol: String,
        /// Number of decimals used to get its user representation.
        /// For example, if `decimals` equals `2`, a balance of `505` coins should
        /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
        decimals: u8,
    }

    /// The initialization of an object with `FungibleSource`.
    public fun init_fungible_source(
        constructor_ref: &ConstructorRef,
        maximum_supply: u64,
        name: String,
        symbol: String,
        decimals: u8,
    ): Object<FungibleSource> {
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
                name,
                symbol,
                decimals,
            }
        );
        object::object_from_constructor_ref<FungibleSource>(constructor_ref)
    }

    /// Self-explanatory.
    public fun get_current_supply<T: key>(asset: &Object<T>): u64 acquires FungibleSource {
        borrow_fungible_source(asset).current_supply
    }

    /// Self-explanatory.
    public fun get_maximum_supply<T: key>(asset: &Object<T>): Option<u64> acquires FungibleSource {
        borrow_fungible_source(asset).maximum_supply
    }

    /// Self-explanatory.
    public fun get_name<T: key>(asset: &Object<T>): String acquires FungibleSource {
        borrow_fungible_source(asset).name
    }

    /// Self-explanatory.
    public fun get_symbol<T: key>(asset: &Object<T>): String acquires FungibleSource {
        borrow_fungible_source(asset).symbol
    }

    /// Self-explanatory.
    public fun get_decimals<T: key>(asset: &Object<T>): u8 acquires FungibleSource {
        borrow_fungible_source(asset).decimals
    }


    /// Increase the supply of a fungible asset by minting.
    public(friend) fun increase_supply<T: key>(asset: &Object<T>, amount: u64) acquires FungibleSource {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        let fungible_source = borrow_fungible_source_mut(asset);
        if (option::is_some(&fungible_source.maximum_supply)) {
            let max = *option::borrow(&fungible_source.maximum_supply);
            assert!(max - fungible_source.current_supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_OVERFLOW))
        };
        fungible_source.current_supply = fungible_source.current_supply + amount;
    }

    /// Increase the supply of a fungible asset by burning.
    public(friend) fun decrease_supply<T: key>(asset: &Object<T>, amount: u64) acquires FungibleSource {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        let fungible_source = borrow_fungible_source_mut(asset);
        assert!(fungible_source.current_supply >= amount, error::invalid_argument(ECURRENT_SUPPLY_UNDERFLOW));
        fungible_source.current_supply = fungible_source.current_supply - amount;
    }


    /// Borrow a `&FungibleSource` from an asset.
    inline fun borrow_fungible_source<T: key>(asset: &Object<T>): &FungibleSource acquires FungibleSource {
        let addr = object::object_address(&verify(asset));
        borrow_global<FungibleSource>(addr)
    }

    /// Borrow a `&mut FungibleSource` from an asset.
    inline fun borrow_fungible_source_mut<T: key>(asset: &Object<T>): &mut FungibleSource acquires FungibleSource {
        let addr = object::object_address(&verify(asset));
        borrow_global_mut<FungibleSource>(addr)
    }

    /// Verify any object is equipped with `FungibleSource` and return its address.
    public fun verify<T: key>(asset: &Object<T>): Object<FungibleSource> {
        let addr = object::object_address(asset);
        object::address_to_object<FungibleSource>(addr)
    }

    #[test_only]
    use std::string;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use std::signer;

    #[test_only]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TestToken has key {}

    #[test_only]
    public fun create_test_token(creator: &signer): (ConstructorRef, Object<TestToken>) {
        account::create_account_for_test(signer::address_of(creator));
        let creator_ref = object::create_object_from_account(creator);
        let object_signer = object::generate_signer(&creator_ref);
        move_to(&object_signer, TestToken {});

        let token = object::object_from_constructor_ref<TestToken>(&creator_ref);
        (creator_ref, token)
    }

    #[test_only]
    public fun init_test_fungible_source(creator_ref: &ConstructorRef): Object<FungibleSource> {
        init_fungible_source(
            creator_ref,
            100 /* max supply */,
            string::utf8(b"USDA"),
            string::utf8(b"$$$"),
            0
        )
    }

    #[test(creator = @0xcafe)]
    fun test_basic_overflow(creator: &signer) acquires FungibleSource {
        let (creator_ref, asset) = create_test_token(creator);
        init_test_fungible_source(&creator_ref);
        assert!(get_current_supply(&asset) == 0, 1);
        assert!(get_maximum_supply(&asset) == option::some(100), 2);
        assert!(get_name(&asset) == string::utf8(b"USDA"), 3);
        assert!(get_symbol(&asset) == string::utf8(b"$$$"), 4);
        assert!(get_decimals(&asset) == 0, 5);

        increase_supply(&asset, 50);
        assert!(get_current_supply(&asset) == 50, 6);
        decrease_supply(&asset, 30);
        assert!(get_current_supply(&asset) == 20, 7);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10002, location = Self)]
    fun test_supply_overflow(creator: &signer) acquires FungibleSource {
        let (creator_ref, asset) = create_test_token(creator);
        init_test_fungible_source(&creator_ref);
        increase_supply(&asset, 101);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10003, location = Self)]
    fun test_supply_underflow(creator: &signer) acquires FungibleSource {
        let (creator_ref, asset) = create_test_token(creator);
        init_test_fungible_source(&creator_ref);
        decrease_supply(&asset, 1);
    }
}
