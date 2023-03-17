/// This is an example showing how to create a fungible asset and how to use it.
module fungible_asset::node_code_coin {
    use aptos_framework::object;
    use std::string::String;
    use fungible_asset::managed_fungible_source::initialize_managing_capabilities;
    use std::string;
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    #[test_only]
    use std::signer::address_of;
    #[test_only]
    use fungible_asset::managed_fungible_source::{mint, burn};
    #[test_only]
    use fungible_asset::fungible_source::transfer;
    #[test_only]
    use fungible_asset::fungible_asset::balance;
    #[test_only]
    use aptos_framework::object::{create_object_address, address_to_object};

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Coin has key {
        name: String,
        symbol: String,
    }

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
        let object_signer = object::generate_signer(&creator_ref);
        move_to(&object_signer, Coin { name, symbol });

        initialize_managing_capabilities(&creator_ref, max_supply, decimals);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    entry fun e2e_test(creator: &signer, aaron: &signer) {
        let usda = string::utf8(b"USDA");
        let creator_address = address_of(creator);
        create_account_for_test(creator_address);
        let aaron_address = address_of(aaron);

        create_coin(creator, usda, string::utf8(b"$"), 0, 0);
        let coin_addr = create_object_address(&creator_address, *string::bytes(&usda));
        let coin = address_to_object<Coin>(coin_addr);


        mint(creator, &coin, 100, aaron_address);
        transfer(aaron, &coin, 80, creator_address);
        burn(creator, &coin, 20, creator_address);
        assert!(balance(creator_address, &coin) == 60, 1);
        assert!(balance(aaron_address, &coin) == 20, 1);
    }
}
