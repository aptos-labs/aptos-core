// Copyright (C) -- Supra 2025
#[test_only]
module 0x2::fungible_asset_tests {
    #[test_only]
    use std::option;
    #[test_only]
    use std::signer;
    #[test_only]
    use std::string;
    #[test_only]
    use supra_framework::account;
    #[test_only]
    use supra_framework::coin;
    #[test_only]
    use supra_framework::fungible_asset;
    #[test_only]
    use supra_framework::object;
    #[test_only]
    use supra_framework::object::{ConstructorRef, Object};
    #[test_only]
    use supra_framework::primary_fungible_store;

    #[test_only]
    struct FakeMoney has key {}

    #[test_only]
    struct TestToken has key {}

    #[test_only]
    public fun create_test_token(creator: &signer): (ConstructorRef, Object<TestToken>) {
        account::create_account_for_test(signer::address_of(creator));
        let creator_ref = object::create_named_object(creator, b"TEST");
        let object_signer = object::generate_signer(&creator_ref);
        move_to(&object_signer, TestToken {});

        let token = object::object_from_constructor_ref<TestToken>(&creator_ref);
        (creator_ref, token)
    }

    #[test(creator1 = @0xcafe, creator2 = @0xface)]
    fun test_metadata_from_different_modules(creator1: &signer, creator2: &signer) {
        let (creator_ref, metadata) = fungible_asset::create_test_token(creator1);
        let (creator_ref_test, metadata_test) = create_test_token(creator2);
        fungible_asset::init_test_metadata(&creator_ref);
        fungible_asset::init_test_metadata(&creator_ref_test);
        assert!(object::object_address(&metadata) != object::object_address(&metadata_test) , 0);
    }

    #[test(account = @supra_framework, account2 = @0x2)]
    fun test_migration_with_existing_primary_fungible_store_two_fakemoney (
        account: &signer,
        account2: &signer
    ) {
        account::create_account_for_test(signer::address_of(account));
        let account_addr = signer::address_of(account);
        account::create_account_for_test(signer::address_of(account2));
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize_and_register_fake_money(account, 1, true);
        let (burn_cap_this, freeze_cap_this, mint_cap_this) = coin::initialize<FakeMoney>(account2, string::utf8(b"Fake money"),
            string::utf8(b"FMD"), 1, true);
        coin::create_pairing<0x1::coin::FakeMoney>(account);
        coin::create_pairing<FakeMoney>(account);
        coin::create_coin_store<FakeMoney>(account);

        let coin = coin::mint<coin::FakeMoney>(50, &mint_cap);
        let coin_this = coin::mint<FakeMoney>(100, &mint_cap_this);

        primary_fungible_store::deposit(account_addr, coin::coin_to_fungible_asset(coin));
        primary_fungible_store::deposit(account_addr, coin::coin_to_fungible_asset(coin_this));

        assert!(coin::balance<0x1::coin::FakeMoney>(account_addr) == 50, 0);
        assert!(coin::balance<FakeMoney>(account_addr) == 100, 0);

        let address = object::object_address(option::borrow(&coin::paired_metadata<0x1::coin::FakeMoney>()));
        let address_this = object::object_address(option::borrow(&coin::paired_metadata<FakeMoney>()));

        assert!(address != address_this, 0);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_burn_cap(burn_cap_this);
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_freeze_cap(freeze_cap_this);
        coin::destroy_mint_cap(mint_cap);
        coin::destroy_mint_cap(mint_cap_this);
    }
}
