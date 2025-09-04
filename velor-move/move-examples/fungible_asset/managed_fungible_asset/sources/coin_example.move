/// A coin example using managed_fungible_asset to create a fungible "coin" and helper functions to only interact with
/// primary fungible stores only.
module example_addr::coin_example {
    use velor_framework::object;
    use velor_framework::fungible_asset::{Self, Metadata, FungibleAsset};
    use velor_framework::object::Object;
    use example_addr::managed_fungible_asset;
    use std::string::utf8;

    const ASSET_SYMBOL: vector<u8> = b"YOLO";

    /// Initialize metadata object and store the refs.
    fun init_module(admin: &signer) {
        let constructor_ref = &object::create_named_object(admin, ASSET_SYMBOL);
        managed_fungible_asset::initialize(
            constructor_ref,
            0, /* maximum_supply. 0 means no maximum */
            utf8(b"You only live once"), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            8, /* decimals */
            utf8(b"http://example.com/favicon.ico"), /* icon */
            utf8(b"http://example.com"), /* project */
            vector[true, true, true], /* mint_ref, transfer_ref, burn_ref */
        );
    }

    #[view]
    /// Return the address of the metadata that's created when this module is deployed.
    public fun get_metadata(): Object<Metadata> {
        let metadata_address = object::create_object_address(&@example_addr, ASSET_SYMBOL);
        object::address_to_object<Metadata>(metadata_address)
    }

    /// Mint as the owner of metadata object.
    public entry fun mint(admin: &signer, to: address, amount: u64) {
        managed_fungible_asset::mint_to_primary_stores(admin, get_metadata(), vector[to], vector[amount]);
    }

    /// Transfer as the owner of metadata object ignoring `frozen` field.
    public entry fun transfer(admin: &signer, from: address, to: address, amount: u64) {
        managed_fungible_asset::transfer_between_primary_stores(
            admin,
            get_metadata(),
            vector[from],
            vector[to],
            vector[amount]
        );
    }

    /// Burn fungible assets as the owner of metadata object.
    public entry fun burn(admin: &signer, from: address, amount: u64) {
        managed_fungible_asset::burn_from_primary_stores(admin, get_metadata(), vector[from], vector[amount]);
    }

    /// Freeze an account so it cannot transfer or receive fungible assets.
    public entry fun freeze_account(admin: &signer, account: address) {
        managed_fungible_asset::set_primary_stores_frozen_status(admin, get_metadata(), vector[account], true);
    }

    /// Unfreeze an account so it can transfer or receive fungible assets.
    public entry fun unfreeze_account(admin: &signer, account: address) {
        managed_fungible_asset::set_primary_stores_frozen_status(admin, get_metadata(), vector[account], false);
    }

    /// Withdraw as the owner of metadata object ignoring `frozen` field.
    public fun withdraw(admin: &signer, from: address, amount: u64): FungibleAsset {
        managed_fungible_asset::withdraw_from_primary_stores(admin, get_metadata(), vector[from], vector[amount])
    }

    /// Deposit as the owner of metadata object ignoring `frozen` field.
    public fun deposit(admin: &signer, fa: FungibleAsset, to: address) {
        let amount = fungible_asset::amount(&fa);
        managed_fungible_asset::deposit_to_primary_stores(
            admin,
            &mut fa,
            vector[to],
            vector[amount]
        );
        fungible_asset::destroy_zero(fa);
    }

    #[test_only]
    use velor_framework::primary_fungible_store;
    #[test_only]
    use std::signer;

    #[test(creator = @example_addr)]
    fun test_basic_flow(creator: &signer) {
        init_module(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        mint(creator, creator_address, 100);
        let metadata = get_metadata();
        assert!(primary_fungible_store::balance(creator_address, metadata) == 100, 4);
        freeze_account(creator, creator_address);
        assert!(primary_fungible_store::is_frozen(creator_address, metadata), 5);
        transfer(creator, creator_address, aaron_address, 10);
        assert!(primary_fungible_store::balance(aaron_address, metadata) == 10, 6);

        unfreeze_account(creator, creator_address);
        assert!(!primary_fungible_store::is_frozen(creator_address, metadata), 7);
        burn(creator, creator_address, 90);
    }

    #[test(creator = @example_addr, aaron = @0xface)]
    #[expected_failure(abort_code = 0x50001, location = example_addr::managed_fungible_asset)]
    fun test_permission_denied(creator: &signer, aaron: &signer) {
        init_module(creator);
        let creator_address = signer::address_of(creator);
        mint(aaron, creator_address, 100);
    }
}
