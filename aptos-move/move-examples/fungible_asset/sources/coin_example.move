/// A coin example using managed_fungible_asset to create a fungible "coin".
module fungible_asset_extension::coin_example {
    use aptos_framework::object;
    use aptos_framework::fungible_asset::{Metadata, FungibleAsset};
    use aptos_framework::object::Object;
    use fungible_asset_extension::managed_fungible_asset;
    use std::string::utf8;

    const ASSET_SYMBOL: vector<u8> = b"YOLO";

    /// Initialize metadata object and store the refs.
    fun init_module(admin: &signer) {
        let constructor_ref = &object::create_named_object(admin, ASSET_SYMBOL);
        managed_fungible_asset::initialize(
            constructor_ref,
            false,
            0, /* maximum_supply. 0 means no maximum */
            utf8(b"You only live once"), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            8, /* decimals */
            utf8(b"http://example.com"), /* icon */
            utf8(b"My issuer"), /* issuer */
        );
    }

    #[view]
    /// Return the address of the metadata that's created when this module is deployed.
    public fun get_metadata(): Object<Metadata> {
        let metadata_address = object::create_object_address(&@fungible_asset_extension, ASSET_SYMBOL);
        object::address_to_object<Metadata>(metadata_address)
    }

    /// Mint as the owner of metadata object.
    public entry fun mint(admin: &signer, amount: u64, to: address) {
        managed_fungible_asset::mint(admin, get_metadata(), amount, to);
    }

    /// Transfer as the owner of metadata object ignoring `frozen` field.
    public entry fun transfer(admin: &signer, from: address, to: address, amount: u64) {
        managed_fungible_asset::transfer(admin, get_metadata(), from, to, amount);
    }

    /// Burn fungible assets as the owner of metadata object.
    public entry fun burn(admin: &signer, from: address, amount: u64) {
        managed_fungible_asset::burn(admin, get_metadata(), from, amount);
    }

    /// Freeze an account so it cannot transfer or receive fungible assets.
    public entry fun freeze_account(admin: &signer, account: address) {
        managed_fungible_asset::freeze_account(admin, get_metadata(), account);
    }

    /// Unfreeze an account so it can transfer or receive fungible assets.
    public entry fun unfreeze_account(admin: &signer, account: address) {
        managed_fungible_asset::unfreeze_account(admin, get_metadata(), account);
    }

    /// Withdraw as the owner of metadata object ignoring `frozen` field.
    public fun withdraw(admin: &signer, amount: u64, from: address): FungibleAsset {
        managed_fungible_asset::withdraw(admin, get_metadata(), amount, from)
    }

    /// Deposit as the owner of metadata object ignoring `frozen` field.
    public fun deposit(admin: &signer, to: address, fa: FungibleAsset) {
        managed_fungible_asset::deposit(admin, get_metadata(), to, fa);
    }

    #[test_only]
    use aptos_framework::primary_fungible_store;
    #[test_only]
    use std::signer;

    #[test(creator = @0xcafe)]
    fun test_basic_flow(creator: &signer) {
        init_module(creator);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        mint(creator, 100, creator_address);
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

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x50001, location = fungible_asset_extension::managed_fungible_asset)]
    fun test_permission_denied(creator: &signer, aaron: &signer) {
        init_module(creator);
        let creator_address = signer::address_of(creator);
        mint(aaron, 100, creator_address);
    }
}
