/// This is an example showing how to create a fungible asset and how to use it.
module fungible_asset::coin {
    use aptos_framework::managed_fungible_metadata;
    use aptos_framework::object;
    use std::string::{Self, String};

    /// Create an coin object with built-in managing capabilities.
    public entry fun create_coin(
        creator: &signer,
        name: String,
        symbol: String,
        max_supply: u64,
        decimals: u8
    ) {
        // TODO(lightmark): create_named_object vs create_object_from_account, which one to choose here.
        let creator_ref = object::create_named_object(creator, *string::bytes(&name));
        managed_fungible_metadata::init_managing_refs(&creator_ref, max_supply, name, symbol, decimals);
    }

    #[test_only]
    use std::signer;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::fungible_store;
    #[test_only]
    use aptos_framework::fungible_asset::FungibleAssetMetadata;

    #[test(creator = @0xcafe, aaron = @0xface)]
    entry fun e2e_test(creator: &signer, aaron: &signer) {
        let usda = string::utf8(b"USDA");
        let creator_address = signer::address_of(creator);
        account::create_account_for_test(creator_address);
        let aaron_address = signer::address_of(aaron);

        create_coin(creator, usda, string::utf8(b"$"), 0, 0);
        let coin_addr = object::create_object_address(&creator_address, *string::bytes(&usda));
        let coin = object::address_to_object<FungibleAssetMetadata>(coin_addr);

        managed_fungible_metadata::mint(creator, &coin, 100, aaron_address);
        fungible_store::transfer(aaron, &coin, 70, creator_address);
        managed_fungible_metadata::set_ungated_transfer(creator, &coin, aaron_address, false);
        managed_fungible_metadata::transfer(creator, &coin, aaron_address, creator_address, 10);
        managed_fungible_metadata::burn(creator, &coin, creator_address, 20);
        assert!(fungible_store::balance(creator_address, &coin) == 60, 1);
        assert!(fungible_store::balance(aaron_address, &coin) == 20, 2);
    }
}
