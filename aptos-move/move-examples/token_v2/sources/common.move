module token_v2::common {
    use std::option::{Self, Option, destroy_some, destroy_none};
    use aptos_framework::object::{Object, object_address, is_owner};
    use std::string::{String, bytes};
    use std::vector;
    use std::string;
    use std::error;
    use std::signer::address_of;

    friend token_v2::collection;
    friend token_v2::token;
    friend token_v2::coin_v2;

    /// The length of cap or ref flags vector is not 3.
    const EFLAGS_INCORRECT_LENGTH: u64 = 1;
    /// Object<T> (Resource T) does not exist.
    const EOBJECT_NOT_FOUND: u64 = 2;
    /// Not the owner.
    const ENOT_OWNER: u64 = 3;
    /// The fungible asset supply exists or does not exist for this asset object.
    const EFUNGIBLE_ASSET_SUPPLY: u64 = 4;
    /// Error about royalty existence.
    const EROYALTY: u64 = 5;
    /// Amount cannot be zero.
    const EZERO_AMOUNT: u64 = 7;
    /// Royalty bps is invalid.
    const EINVALID_BASIS_POINTS: u64 = 8;
    /// Name is invalid.
    const EINVALID_NAME: u64 = 9;
    /// The current_supply of token as fungible assets is not zero.
    const ECURRENT_SUPPLY_NON_ZERO: u64 = 10;
    /// Mint capability exists or does not exist.
    const EMINT_CAP: u64 = 11;
    /// Freeze capability exists does not exist.
    const EFREEZE_CAP: u64 = 12;
    /// Burn capability exists or does not exist.
    const EBURN_CAP: u64 = 13;
    /// Current supply overflow
    const ECURRENT_SUPPLY_OVERFLOW: u64 = 14;
    /// Current supply underflow
    const ECURRENT_SUPPLY_UNDERFLOW: u64 = 15;
    /// The asset owner caps have to have mint capability.
    const ECAP_FLAGS_WITH_NO_MINT: u64 = 16;
    /// Max supply cannot be zero if exists.
    const EZERO_MAX_SUPPLY: u64 = 17;

    public fun assert_flags_length(flags: &vector<bool>) {
        assert!(vector::length(flags) == 3, error::invalid_argument(EFLAGS_INCORRECT_LENGTH));
    }

    public fun assert_valid_name(name: &String) {
        assert!(is_valid_name(name), error::invalid_argument(EINVALID_NAME));
    }

    public fun assert_owner<T: key>(owner: &signer, asset: &Object<T>) {
        assert!(is_owner(*asset, address_of(owner)), error::permission_denied(ENOT_OWNER));
    }

    /// Only allow human readable characters in naming.
    fun is_valid_name(name: &String): bool {
        if (string::is_empty(name)) {
            return false
        };
        std::vector::all(bytes(name), |char| *char >= 32 && *char <= 126)
    }

    // ================================================================================================================
    // Royalty
    // ================================================================================================================
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The royalty of a token within this collection -- this optional
    struct Royalty has copy, drop, key {
        // The bps of sale price considered as royalty.
        bps: u32,
        /// The recipient of royalty payments. See the `shared_account` for how to handle multiple
        /// creators.
        payee_address: address,
    }

    public(friend) fun royalty_new(bps: u32, payee_address: address): Royalty {
        assert!(bps <= 10000, error::invalid_argument(EINVALID_BASIS_POINTS));
        Royalty { bps, payee_address }
    }

    public(friend) fun init_royalty(object_signer: &signer, royalty: Royalty) {
        move_to(object_signer, royalty);
    }

    public(friend) fun remove_royalty(object_address: address) acquires Royalty {
        move_from<Royalty>(object_address);
    }

    public fun royalty_exists(object_address: address): bool {
        exists<Royalty>(object_address)
    }

    public fun get_royalty(object_address: address): Royalty acquires Royalty {
        assert!(royalty_exists(object_address), error::not_found(EROYALTY));
        *borrow_global<Royalty>(object_address)
    }

    public fun get_royalty_pencentage(royalty: &Royalty): u32 {
        royalty.bps
    }

    public fun get_royalty_payee_address(royalty: &Royalty): address {
        royalty.payee_address
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct FungibleAssetMetadata has key {
        supply: Supply,
        asset_owner_caps: AssetOwnerCaps
    }

    struct Supply has copy, drop, store {
        current: u64,
        maximum: Option<u64>
    }

    public fun supply_new(maximum: Option<u64>): Supply {
        if (option::is_some(&maximum)) {
            assert!(*option::borrow(&maximum) != 0, error::invalid_argument(EZERO_MAX_SUPPLY));
        };
        Supply {
            current: 0,
            maximum
        }
    }

    public fun get_current_supply<T: key>(asset: &Object<T>): u64 acquires FungibleAssetMetadata {
        let supply = borrow_supply(asset);
        supply.current
    }

    public fun get_maximum_supply<T: key>(asset: &Object<T>): Option<u64> acquires FungibleAssetMetadata {
        let supply = borrow_supply(asset);
        supply.maximum
    }

    /// maximum = 0 means no maximum limit.
    public fun init_fungible_asset_metadata(object_signer: &signer, supply: Supply) {
        move_to(object_signer,
            FungibleAssetMetadata {
                supply,
                asset_owner_caps: new_asset_owner_caps(address_of(object_signer))
            }
        );
    }

    public fun assert_fungible_asset_metadata_exists<T: key>(asset: &Object<T>) {
        assert!(fungible_asset_metadata_exists(asset), error::not_found(EFUNGIBLE_ASSET_SUPPLY));
    }

    public fun assert_fungible_asset_metadata_not_exists<T: key>(asset: &Object<T>) {
        assert!(!fungible_asset_metadata_exists(asset), error::already_exists(EFUNGIBLE_ASSET_SUPPLY));
    }

    public fun fungible_asset_metadata_exists<T: key>(asset: &Object<T>): bool {
        exists<FungibleAssetMetadata>(object_address(asset))
    }

    /// When supply goes down to 0, `BurnCap` can remove the fungibility of the asset.
    public fun remove_fungible_asset_metadata<T: key>(
        asset: &Object<T>
    ) acquires FungibleAssetMetadata {
        let FungibleAssetMetadata {
            supply,
            asset_owner_caps
        } = move_from<FungibleAssetMetadata>(object_address(asset));
        assert!(supply.current == 0, error::permission_denied(ECURRENT_SUPPLY_NON_ZERO));

        let AssetOwnerCaps { mint, freeze, burn } = asset_owner_caps;
        let has_mint = option::is_some(&mint);
        let has_freeze = option::is_some(&freeze);
        let has_burn = option::is_some(&burn);
        if (has_mint) {
            destory_mint_cap(destroy_some(mint));
        } else {
            destroy_none(mint);
        };
        if (has_freeze) {
            destory_freeze_cap(destroy_some(freeze));
        } else {
            destroy_none(freeze);
        };
        if (has_burn) {
            destory_burn_cap(destroy_some(burn));
        } else {
            destroy_none(burn);
        };
    }

    /// Borrow a `&Supply` from a fungible asset.
    inline fun borrow_supply<T: key>(asset: &Object<T>): &Supply acquires FungibleAssetMetadata {
        assert_fungible_asset_metadata_exists(asset);
        let object_addr = object_address(asset);
        &borrow_global<FungibleAssetMetadata>(object_addr).supply
    }

    /// Borrow a `&mut Supply` from a fungible asset.
    inline fun borrow_supply_mut<T: key>(asset: &Object<T>): &mut Supply acquires FungibleAssetMetadata {
        assert_fungible_asset_metadata_exists(asset);
        let object_addr = object_address(asset);
        &mut borrow_global_mut<FungibleAssetMetadata>(object_addr).supply
    }

    /// Increase the supply of a fungible asset by minting.
    public fun increase_supply<T: key>(cap: &MintCap, asset: &Object<T>, amount: u64) acquires FungibleAssetMetadata {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        assert_mint_cap_and_asset_match(cap, asset);
        let supply = borrow_supply_mut(asset);
        if (option::is_some(&supply.maximum)) {
            let max = *option::borrow(&supply.maximum);
            assert!(max - supply.current >= amount, error::invalid_argument(ECURRENT_SUPPLY_OVERFLOW))
        };
        supply.current = supply.current + amount;
    }

    /// Increase the supply of a fungible asset by burning.
    public fun decrease_supply<T: key>(cap: &BurnCap, asset: &Object<T>, amount: u64) acquires FungibleAssetMetadata {
        assert!(amount != 0, error::invalid_argument(EZERO_AMOUNT));
        assert_burn_cap_and_asset_match(cap, asset);
        let supply = borrow_supply_mut(asset);
        assert!(supply.current >= amount, error::invalid_argument(ECURRENT_SUPPLY_UNDERFLOW));
        supply.current = supply.current - amount;
    }

    /// Capability functions
    struct MintCap has store {
        asset_addr: address
    }

    struct FreezeCap has store {
        asset_addr: address
    }

    struct BurnCap has store {
        asset_addr: address
    }

    struct AssetOwnerCaps has store {
        mint: Option<MintCap>,
        freeze: Option<FreezeCap>,
        burn: Option<BurnCap>,
    }

    public fun new_asset_owner_caps(asset_addr: address): AssetOwnerCaps {
        AssetOwnerCaps {
            mint: option::some(MintCap { asset_addr }),
            freeze: option::some(FreezeCap { asset_addr }),
            burn: option::some(BurnCap { asset_addr })
        }
    }

    inline fun borrow_asset_owner_caps<T: key>(asset: &Object<T>): &AssetOwnerCaps acquires FungibleAssetMetadata {
        assert_fungible_asset_metadata_exists(asset);
        &borrow_global<FungibleAssetMetadata>(object_address(asset)).asset_owner_caps
    }

    inline fun borrow_asset_owner_caps_mut<T: key>(
        owner: &signer,
        asset: &Object<T>
    ): &mut AssetOwnerCaps acquires FungibleAssetMetadata {
        assert_owner(owner, asset);
        assert_fungible_asset_metadata_exists(asset);
        &mut borrow_global_mut<FungibleAssetMetadata>(object_address(asset)).asset_owner_caps
    }

    public fun asset_owner_caps_contain_mint<T: key>(asset: &Object<T>): bool acquires FungibleAssetMetadata {
        option::is_some(&borrow_asset_owner_caps(asset).mint)
    }

    public fun asset_owner_caps_contain_freeze<T: key>(asset: &Object<T>): bool acquires FungibleAssetMetadata {
        option::is_some(&borrow_asset_owner_caps(asset).freeze)
    }

    public fun asset_owner_caps_contain_burn<T: key>(asset: &Object<T>): bool acquires FungibleAssetMetadata {
        option::is_some(&borrow_asset_owner_caps(asset).burn)
    }

    public(friend) fun destory_mint_cap(cap: MintCap) {
        let MintCap { asset_addr: _ } = cap;
    }

    public(friend) fun destory_freeze_cap(cap: FreezeCap) {
        let FreezeCap { asset_addr: _ } = cap;
    }

    public(friend) fun destory_burn_cap(cap: BurnCap) {
        let BurnCap { asset_addr: _ } = cap;
    }

    public(friend) fun get_mint_from_asset_owner_caps<T: key>(
        owner: &signer,
        asset: &Object<T>
    ): MintCap acquires FungibleAssetMetadata {
        let mint_cap = &mut borrow_asset_owner_caps_mut(owner, asset).mint;
        assert!(option::is_some(mint_cap), error::not_found(EMINT_CAP));
        option::extract(mint_cap)
    }

    public(friend) fun get_freeze_from_asset_owner_caps<T: key>(
        owner: &signer,
        asset: &Object<T>
    ): FreezeCap acquires FungibleAssetMetadata {
        let freeze_cap = &mut borrow_asset_owner_caps_mut(owner, asset).freeze;
        assert!(option::is_some(freeze_cap), error::not_found(EFREEZE_CAP));
        option::extract(freeze_cap)
    }

    public(friend) fun get_burn_from_asset_owner_caps<T: key>(
        owner: &signer,
        asset: &Object<T>
    ): BurnCap acquires FungibleAssetMetadata {
        let burn_cap = &mut borrow_asset_owner_caps_mut(owner, asset).burn;
        assert!(option::is_some(burn_cap), error::not_found(EBURN_CAP));
        option::extract(burn_cap)
    }

    public(friend) fun put_mint_to_asset_owner_caps<T: key>(
        owner: &signer,
        asset: &Object<T>,
        cap: MintCap
    ) acquires FungibleAssetMetadata {
        assert_mint_cap_and_asset_match(&cap, asset);
        let mint_cap = &mut borrow_asset_owner_caps_mut(owner, asset).mint;
        assert!(option::is_none(mint_cap), error::already_exists(EMINT_CAP));
        option::fill(mint_cap, cap);
    }

    public(friend) fun put_freeze_to_asset_owner_caps<T: key>(
        owner: &signer,
        asset: &Object<T>,
        cap: FreezeCap
    ) acquires FungibleAssetMetadata {
        assert_freeze_cap_and_asset_match(&cap, asset);
        let freeze_cap = &mut borrow_asset_owner_caps_mut(owner, asset).freeze;
        assert!(option::is_none(freeze_cap), error::already_exists(EFREEZE_CAP));
        option::fill(freeze_cap, cap);
    }

    public(friend) fun put_burn_to_asset_owner_caps<T: key>(
        owner: &signer,
        asset: &Object<T>,
        cap: BurnCap
    ) acquires FungibleAssetMetadata {
        assert_burn_cap_and_asset_match(&cap, asset);
        let burn_cap = &mut borrow_asset_owner_caps_mut(owner, asset).burn;
        assert!(option::is_none(burn_cap), error::already_exists(EBURN_CAP));
        option::fill(burn_cap, cap);
    }


    public fun assert_mint_cap_and_asset_match<T: key>(cap: &MintCap, asset: &Object<T>) {
        assert!(cap.asset_addr == object_address(asset), error::invalid_argument(EMINT_CAP));
    }

    public fun assert_freeze_cap_and_asset_match<T: key>(cap: &FreezeCap, asset: &Object<T>) {
        assert!(cap.asset_addr == object_address(asset), error::invalid_argument(EFREEZE_CAP));
    }

    public fun assert_burn_cap_and_asset_match<T: key>(cap: &BurnCap, asset: &Object<T>) {
        assert!(cap.asset_addr == object_address(asset), error::invalid_argument(EBURN_CAP));
    }

    #[test_only]
    use aptos_framework::object::{Self, ConstructorRef};

    #[test_only]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TestToken has key {}

    #[test_only]
    fun new_mint_cap(asset_addr: address): MintCap {
        MintCap { asset_addr }
    }

    #[test_only]
    fun new_burn_cap(asset_addr: address): BurnCap {
        BurnCap { asset_addr }
    }

    #[test_only]
    fun destrop_mint_cap(mint_cap: MintCap) {
        let MintCap { asset_addr: _ } = mint_cap;
    }

    #[test_only]
    fun destroy_burn_cap(burn_cap: BurnCap) {
        BurnCap { asset_addr: _ } = burn_cap;
    }

    #[test_only]
    public fun create_test_token(creator: &signer): (ConstructorRef, Object<TestToken>) {
        let creator_ref = object::create_object_from_account(creator);
        let object_signer = object::generate_signer(&creator_ref);
        move_to(&object_signer, TestToken {});

        let token = object::object_from_constructor_ref<TestToken>(&creator_ref);
        (creator_ref, token)
    }

    #[test_only]
    public fun create_and_initialize_test_token(creator: &signer): Object<TestToken> {
        let (creator_ref, token_obj) = create_test_token(creator);
        let object_signer = object::generate_signer(&creator_ref);
        let supply = supply_new(option::none());
        assert_fungible_asset_metadata_not_exists(&token_obj);
        init_fungible_asset_metadata(&object_signer, supply);
        token_obj
    }

    #[test(creator = @0xcafe)]
    fun test_royalty(creator: &signer) acquires Royalty {
        let (creator_ref, _token_obj) = create_test_token(creator);
        let royalty = royalty_new(10, @0xface);
        let object_signer = object::generate_signer(&creator_ref);
        let token_address = address_of(&object_signer);
        assert!(!royalty_exists(token_address), 0);
        init_royalty(&object_signer, royalty);
        assert!(royalty_exists(token_address), 1);
        let royalty = get_royalty(token_address);
        assert!(get_royalty_pencentage(&royalty) == 10, 2);
        assert!(get_royalty_payee_address(&royalty) == @0xface, 3);
        remove_royalty(token_address);
        assert!(!royalty_exists(token_address), 4);
    }

    #[test(creator = @0xcafe)]
    fun test_fungible_asset_metadata(creator: &signer) acquires FungibleAssetMetadata {
        let token_obj = create_and_initialize_test_token(creator);
        assert!(get_current_supply(&token_obj) == 0, 0);
        assert!(option::is_none(&get_maximum_supply(&token_obj)), 1);
        assert!(asset_owner_caps_contain_mint(&token_obj), 2);
        assert!(asset_owner_caps_contain_freeze(&token_obj), 3);
        assert!(asset_owner_caps_contain_burn(&token_obj), 4);

        let mint_cap = get_mint_from_asset_owner_caps(creator, &token_obj);
        let freeze_cap = get_freeze_from_asset_owner_caps(creator, &token_obj);
        let burn_cap = get_burn_from_asset_owner_caps(creator, &token_obj);

        assert!(!asset_owner_caps_contain_mint(&token_obj), 5);
        assert!(!asset_owner_caps_contain_freeze(&token_obj), 6);
        assert!(!asset_owner_caps_contain_burn(&token_obj), 7);

        increase_supply(&mint_cap, &token_obj, 100);
        assert!(get_current_supply(&token_obj) == 100, 0);
        decrease_supply(&burn_cap, &token_obj, 100);

        put_mint_to_asset_owner_caps(creator, &token_obj, mint_cap);
        put_freeze_to_asset_owner_caps(creator, &token_obj, freeze_cap);
        put_burn_to_asset_owner_caps(creator, &token_obj, burn_cap);

        remove_fungible_asset_metadata(&token_obj);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10011, location = Self)]
    fun test_bad_mint_cap(creator: &signer) acquires FungibleAssetMetadata {
        let token_obj = create_and_initialize_test_token(creator);
        let fake_mint_cap = new_mint_cap(@0x0);
        increase_supply(&fake_mint_cap, &token_obj, 100);
        destory_mint_cap(fake_mint_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10014, location = Self)]
    fun test_max_supply_overflow(creator: &signer) acquires FungibleAssetMetadata {
        let (creator_ref, token_obj) = create_test_token(creator);
        let object_signer = object::generate_signer(&creator_ref);
        let supply = supply_new(option::some(1));
        init_fungible_asset_metadata(&object_signer, supply);
        let mint_cap = get_mint_from_asset_owner_caps(creator, &token_obj);
        increase_supply(&mint_cap, &token_obj, 2);
        destory_mint_cap(mint_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10015, location = Self)]
    fun test_supply_underflow(creator: &signer) acquires FungibleAssetMetadata {
        let token_obj = create_and_initialize_test_token(creator);
        let burn_cap = get_burn_from_asset_owner_caps(creator, &token_obj);
        decrease_supply(&burn_cap, &token_obj, 1);
        destory_burn_cap(burn_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x10013, location = Self)]
    fun test_bad_burn_cap(creator: &signer) acquires FungibleAssetMetadata {
        let token_obj = create_and_initialize_test_token(creator);
        let fake_burn_cap = new_burn_cap(@0x0);
        let mint_cap = get_mint_from_asset_owner_caps(creator, &token_obj);
        increase_supply(&mint_cap, &token_obj, 100);
        decrease_supply(&fake_burn_cap, &token_obj, 100);
        destory_burn_cap(fake_burn_cap);
        destory_mint_cap(mint_cap);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x50010, location = Self)]
    fun test_failed_metadata_removal_by_non_zero_supply(creator: &signer) acquires FungibleAssetMetadata {
        let token_obj = create_and_initialize_test_token(creator);
        let mint_cap = get_mint_from_asset_owner_caps(creator, &token_obj);
        increase_supply(&mint_cap, &token_obj, 100);
        put_mint_to_asset_owner_caps(creator, &token_obj, mint_cap);
        remove_fungible_asset_metadata(&token_obj)
    }
}
