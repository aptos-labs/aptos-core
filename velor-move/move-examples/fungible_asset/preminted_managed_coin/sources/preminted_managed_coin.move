/// This module shows an example how to issue preminted coin with only `transfer` and `burn` managing capabilities.
/// It leveraged `managed_fungible_asset` module with only `TransferRef` and `BurnRef` stored after pre-minting a
/// pre-defined totally supply to a reserve account. After the initialization, the total supply can increase by no means
/// since `MintRef` of this fungible asset does not exist anymore.
/// The `init_module()` code can be modified to customize the managing refs as needed.
module example_addr::preminted_managed_coin {
    use velor_framework::fungible_asset::{Self, Metadata};
    use velor_framework::object::{Self, Object};
    use velor_framework::primary_fungible_store;
    use example_addr::managed_fungible_asset;
    use std::signer;
    use std::string::utf8;

    const ASSET_SYMBOL: vector<u8> = b"MEME";
    const PRE_MINTED_TOTAL_SUPPLY: u64 = 10000;

    /// Initialize metadata object and store the refs.
    fun init_module(admin: &signer) {
        let constructor_ref = &object::create_named_object(admin, ASSET_SYMBOL);
        managed_fungible_asset::initialize(
            constructor_ref,
            1000000000, /* maximum_supply */
            utf8(b"preminted coin"), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            8, /* decimals */
            utf8(b"http://example.com/favicon.ico"), /* icon */
            utf8(b"http://example.com"), /* project */
            vector[false, true, true], /* mint_ref, transfer_ref, burn_ref */
        );

        // Create mint ref to premint fungible asset with a fixed supply volume into a specific account.
        // This account can be any account including normal user account, resource account, multi-sig account, etc.
        // We just use the creator account to show the proof of concept.
        let mint_ref = fungible_asset::generate_mint_ref(constructor_ref);
        let admin_primary_store = primary_fungible_store::ensure_primary_store_exists(
            signer::address_of(admin),
            get_metadata()
        );
        fungible_asset::mint_to(&mint_ref, admin_primary_store, PRE_MINTED_TOTAL_SUPPLY);
    }

    #[view]
    /// Return the address of the metadata that's created when this module is deployed.
    /// This function is optional as a helper function for offline applications.
    public fun get_metadata(): Object<Metadata> {
        let metadata_address = object::create_object_address(&@example_addr, ASSET_SYMBOL);
        object::address_to_object<Metadata>(metadata_address)
    }

    #[test_only]
    use std::option;

    #[test(creator = @example_addr)]
    #[expected_failure(abort_code = 0x60004, location = example_addr::managed_fungible_asset)]
    fun test_basic_flow(creator: &signer) {
        init_module(creator);
        let creator_address = signer::address_of(creator);
        let metadata = get_metadata();

        assert!(option::destroy_some(fungible_asset::supply(metadata)) == (PRE_MINTED_TOTAL_SUPPLY as u128), 1);
        managed_fungible_asset::mint_to_primary_stores(creator, metadata, vector[creator_address], vector[100]);
    }
}
