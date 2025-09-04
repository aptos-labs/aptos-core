script {
    use std::signer;
    use std::string::{utf8};
    use std::option;

    use velor_framework::object::{Self, object_from_constructor_ref};
    use velor_framework::fungible_asset::{Self, Metadata};
    use velor_framework::primary_fungible_store::{Self};

    const ASSET_SYMBOL: vector<u8> = b"FA";

    fun main(admin: &signer) {
        let constructor_ref = &object::create_named_object(admin, ASSET_SYMBOL);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            constructor_ref,
            option::none(),
            utf8(b"FA Coin"), /* name */
            utf8(ASSET_SYMBOL), /* symbol */
            8, /* decimals */
            utf8(b"https://example.com/favicon.ico"), /* icon */
            utf8(b"https://example.com"), /* project */
        );
        let metadata = object_from_constructor_ref<Metadata>(constructor_ref);

        // Create mint/burn/transfer refs to allow creator to manage the fungible asset.
        let mint_ref = fungible_asset::generate_mint_ref(constructor_ref);
        let burn_ref = fungible_asset::generate_burn_ref(constructor_ref);
        let transfer_ref = fungible_asset::generate_transfer_ref(constructor_ref);

        // Mint FA
        let amount = 1000000;
        let fa = fungible_asset::mint(&mint_ref, amount);

        // Transfer FA
        let fa_store = primary_fungible_store::ensure_primary_store_exists(signer::address_of(admin), metadata);
        fungible_asset::deposit_with_ref(&transfer_ref, fa_store, fa);

        // Burn FA
        fungible_asset::burn_from(&burn_ref, fa_store, amount);
    }
}
