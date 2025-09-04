#[test_only]
module marketplace::test_utils {
    use std::signer;
    use std::string;
    use std::vector;

    use velor_framework::account;
    use velor_framework::velor_coin::{Self, VelorCoin};
    use velor_framework::coin;
    use velor_framework::object::{Self, Object};
    use velor_framework::timestamp;

    use velor_token::token as tokenv1;
    use velor_token_objects::token::Token;
    use velor_token_objects::velor_token;
    use velor_token_objects::collection::Collection;

    use marketplace::fee_schedule::{Self, FeeSchedule};

    public inline fun setup(
        velor_framework: &signer,
        marketplace: &signer,
        seller: &signer,
        purchaser: &signer,
    ): (address, address, address) {
        timestamp::set_time_has_started_for_testing(velor_framework);
        let (burn_cap, mint_cap) = velor_coin::initialize_for_test(velor_framework);

        let marketplace_addr = signer::address_of(marketplace);
        account::create_account_for_test(marketplace_addr);
        coin::register<VelorCoin>(marketplace);

        let seller_addr = signer::address_of(seller);
        account::create_account_for_test(seller_addr);
        coin::register<VelorCoin>(seller);

        let purchaser_addr = signer::address_of(purchaser);
        account::create_account_for_test(purchaser_addr);
        coin::register<VelorCoin>(purchaser);

        let coins = coin::mint(10000, &mint_cap);
        coin::deposit(seller_addr, coins);
        let coins = coin::mint(10000, &mint_cap);
        coin::deposit(purchaser_addr, coins);

        coin::destroy_burn_cap(burn_cap);
        coin::destroy_mint_cap(mint_cap);

        (marketplace_addr, seller_addr, purchaser_addr)
    }

    public fun fee_schedule(seller: &signer): Object<FeeSchedule> {
        fee_schedule::init(
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

    public fun mint_tokenv2_with_collection(seller: &signer): (Object<Collection>, Object<Token>) {
        let collection_name = string::utf8(b"collection_name");

        let collection_object = velor_token::create_collection_object(
            seller,
            string::utf8(b"collection description"),
            2,
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

        let velor_token = velor_token::mint_token_object(
            seller,
            collection_name,
            string::utf8(b"description"),
            string::utf8(b"token_name"),
            string::utf8(b"uri"),
            vector::empty(),
            vector::empty(),
            vector::empty(),
        );
        (object::convert(collection_object), object::convert(velor_token))
    }

    public fun mint_tokenv2_with_collection_royalty(
        seller: &signer,
        royalty_numerator: u64,
        royalty_denominator: u64
    ): (Object<Collection>, Object<Token>) {
        let collection_name = string::utf8(b"collection_name");

        let collection_object = velor_token::create_collection_object(
            seller,
            string::utf8(b"collection description"),
            2,
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
            royalty_numerator,
            royalty_denominator,
        );

        let velor_token = velor_token::mint_token_object(
            seller,
            collection_name,
            string::utf8(b"description"),
            string::utf8(b"token_name"),
            string::utf8(b"uri"),
            vector::empty(),
            vector::empty(),
            vector::empty(),
        );
        (object::convert(collection_object), object::convert(velor_token))
    }

    public fun mint_tokenv2(seller: &signer): Object<Token> {
        let (_collection, token) = mint_tokenv2_with_collection(seller);
        token
    }

    public fun mint_tokenv2_additional(seller: &signer): Object<Token> {
        let collection_name = string::utf8(b"collection_name");

        let velor_token = velor_token::mint_token_object(
            seller,
            collection_name,
            string::utf8(b"description"),
            string::utf8(b"token_name_2"),
            string::utf8(b"uri"),
            vector::empty(),
            vector::empty(),
            vector::empty(),
        );
        object::convert(velor_token)
    }

    public fun mint_tokenv1(seller: &signer): tokenv1::TokenId {
        let collection_name = string::utf8(b"collection_name");
        let token_name = string::utf8(b"token_name");

        tokenv1::create_collection(
            seller,
            collection_name,
            string::utf8(b"Collection: Hello, World"),
            string::utf8(b"https://velor.dev"),
            2,
            vector[true, true, true],
        );

        tokenv1::create_token_script(
            seller,
            collection_name,
            token_name,
            string::utf8(b"Hello, Token"),
            1,
            1,
            string::utf8(b"https://velor.dev"),
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

    public fun mint_tokenv1_additional(seller: &signer): tokenv1::TokenId {
        let collection_name = string::utf8(b"collection_name");
        let token_name = string::utf8(b"token_name_2");
        tokenv1::create_token_script(
            seller,
            collection_name,
            token_name,
            string::utf8(b"Hello, Token"),
            1,
            1,
            string::utf8(b"https://velor.dev"),
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

    public fun mint_tokenv1_additional_royalty(
        seller: &signer,
        royalty_numerator: u64,
        royalty_denominator: u64
    ): tokenv1::TokenId {
        let collection_name = string::utf8(b"collection_name");
        let token_name = string::utf8(b"token_name_2");
        tokenv1::create_token_script(
            seller,
            collection_name,
            token_name,
            string::utf8(b"Hello, Token"),
            1,
            1,
            string::utf8(b"https://velor.dev"),
            signer::address_of(seller),
            royalty_denominator,
            royalty_numerator,
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
