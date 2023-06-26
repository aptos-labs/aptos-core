#[test_only]
module marketplace::test_utils {
    use std::signer;
    use std::string;
    use std::vector;

    use aptos_framework::account;
    use aptos_framework::aptos_coin::{Self, AptosCoin};
    use aptos_framework::coin;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::timestamp;

    use aptos_token::token as tokenv1;
    use aptos_token_objects::token::Token;
    use aptos_token_objects::aptos_token;

    use marketplace::fee_schedule::{Self, FeeSchedule};

    public inline fun setup(
        aptos_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ): (address, address, address) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        let marketplace_addr = signer::address_of(marketplace);
        account::create_account_for_test(marketplace_addr);
        coin::register<AptosCoin>(marketplace);

        let seller_addr = signer::address_of(seller);
        account::create_account_for_test(seller_addr);
        coin::register<AptosCoin>(seller);

        let purchaser_addr = signer::address_of(purchaser);
        account::create_account_for_test(purchaser_addr);
        coin::register<AptosCoin>(purchaser);

        let coins = coin::mint(10000, &mint_cap);
        coin::deposit(seller_addr, coins);
        let coins = coin::mint(10000, &mint_cap);
        coin::deposit(purchaser_addr, coins);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);

        (marketplace_addr, seller_addr, purchaser_addr)
    }

    public fun fee_schedule(seller: &signer): Object<FeeSchedule> {
        fee_schedule::init_internal(
            seller,
            signer::address_of(seller),
            2,
            1,
            100,
            1,
        )
    }

    public inline fun increment_timestamp(seconds: u64) {
        timestamp::update_global_time_for_test(timestamp::now_microseconds() + (seconds * 1000000));
    }

    public fun mint_tokenv2(seller: &signer): Object<Token> {
        let seller_addr = signer::address_of(seller);
        let collection_name = string::utf8(b"collection_name");
        let token_creation_num = account::get_guid_next_creation_num(seller_addr);

        aptos_token::create_collection(
            seller,
            string::utf8(b"collection description"),
            1,
            collection_name,
            string::utf8(b"collection uri"),
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            1,
            100,
        );

        aptos_token::mint(
            seller,
            collection_name,
            string::utf8(b"description"),
            string::utf8(b"token_name"),
            string::utf8(b"uri"),
            vector::empty(),
            vector::empty(),
            vector::empty(),
        );

        let obj_addr = object::create_guid_object_address(seller_addr, token_creation_num);
        object::address_to_object(obj_addr)
    }

    public fun mint_tokenv1(seller: &signer): tokenv1::TokenId {
        let collection_name = string::utf8(b"collection_name");
        let token_name = string::utf8(b"token_name");

        tokenv1::create_collection(
            seller,
            collection_name,
            string::utf8(b"Collection: Hello, World"),
            string::utf8(b"https://aptos.dev"),
            1,
            vector[true, true, true],
        );

        tokenv1::create_token_script(
            seller,
            collection_name,
            token_name,
            string::utf8(b"Hello, Token"),
            1,
            1,
            string::utf8(b"https://aptos.dev"),
            signer::address_of(seller),
            100,
            1,
            vector[true, true, true, true, true],
            vector::empty(),
            vector::empty(),
            vector::empty(),
        );

        tokenv1::create_token_id_raw(
            signer::address_of(seller),
            collection_name,
            token_name,
            0,
        )
    }
}
