/// An example combining fungible assets with token as fungible token. In this example, a token object is used as
/// metadata to create fungible units, aka, fungible tokens.
module example_addr::managed_fungible_token {
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::{Self, Object};
    use std::signer;
    use std::string::{utf8, String};
    use std::option;
    use aptos_token_objects::token::{create_named_token, create_token_seed};
    use aptos_token_objects::collection::create_fixed_collection;
    use example_addr::managed_fungible_asset;

    const ASSET_SYMBOL: vector<u8> = b"YOLO";

    /// The signer is not the module's account.
    const ENOT_AUTHORIZED: u64 = 1;

    /// Initialize metadata object and store the refs.
    public entry fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @example_addr, ENOT_AUTHORIZED);
        let collection_name: String = utf8(b"test collection name");
        let token_name: String = utf8(b"test token name");
        create_fixed_collection(
            admin,
            utf8(b"test collection description"),
            1,
            collection_name,
            option::none(),
            utf8(b"http://aptoslabs.com/collection"),
        );
        let constructor_ref = &create_named_token(admin,
            collection_name,
            utf8(b"test token description"),
            token_name,
            option::none(),
            utf8(b"http://aptoslabs.com/token"),
        );

        managed_fungible_asset::initialize(
            constructor_ref,
            0, /* maximum_supply. 0 means no maximum */
            utf8(b"test fungible token"), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            0, /* decimals */
            utf8(b"http://example.com/favicon.ico"), /* icon */
            utf8(b"http://example.com"), /* project */
            vector[true, true, true], /* mint_ref, transfer_ref, burn_ref */
        );
    }

    #[view]
    /// Return the address of the managed fungible asset that's created when this module is deployed.
    /// This function is optional as a helper function for offline applications.
    public fun get_metadata(): Object<Metadata> {
        let collection_name: String = utf8(b"test collection name");
        let token_name: String = utf8(b"test token name");
        let asset_address = object::create_object_address(
            &@example_addr,
            create_token_seed(&collection_name, &token_name)
        );
        object::address_to_object<Metadata>(asset_address)
    }

    #[test(creator = @example_addr)]
    fun test_init(creator: &signer) {
        initialize(creator);
    }
}
