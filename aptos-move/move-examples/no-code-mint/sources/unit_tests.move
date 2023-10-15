#[test_only]
module no_code_mint::unit_tests {
    use std::string::{String, utf8 as str};
    use no_code_mint::allowlist;
    use no_code_mint::package_manager;
    use no_code_mint::mint_machine;
    use std::account;
    use std::bcs;
    use std::vector;
    use std::signer;
    use aptos_token_objects::aptos_token::{AptosToken};
    use std::timestamp;
    use aptos_std::object;
    use aptos_std::aptos_coin::{AptosCoin};
    use aptos_std::coin;
    use aptos_token_objects::token::{Self, Token};
    const COLLECTION_DESCRIPTION: vector<u8> = b"Your collection description here!";
    const TOKEN_DESCRIPTION: vector<u8> = b"Your token description here!";
    const MUTABLE_COLLECTION_DESCRIPTION: bool = false;
    const MUTABLE_ROYALTY: bool = false;
    const MUTABLE_URI: bool = false;
    const MUTABLE_TOKEN_DESCRIPTION: bool = false;
    const MUTABLE_TOKEN_NAME: bool = false;
    const MUTABLE_TOKEN_PROPERTIES: bool = true;
    const MUTABLE_TOKEN_URI: bool = false;
    const TOKENS_BURNABLE_BY_CREATOR: bool = false;
    const TOKENS_FREEZABLE_BY_CREATOR: bool = false;
    const MINTER_STARTING_COINS: u64 = 100;
    const COLLECTION_NAME: vector<u8> = b"Krazy Kangaroos";
    const TOKEN_BASE_NAME: vector<u8> = b"Krazy Kangaroo #";
    const TOKEN_BASE_URI: vector<u8> = b"https://arweave.net/";
    const COLLECTION_URI: vector<u8> = b"https://www.link-to-your-collection-image.com";
    const ROYALTY_NUMERATOR: u64 = 5;
    const ROYALTY_DENOMINATOR: u64 = 100;
    const MAX_SUPPLY: u64 = 100;
    const START_TIMESTAMP_PUBLIC: u64 = 100000000;
    const START_TIMESTAMP_WHITELIST: u64 = 100000000 - 1;
    const END_TIMESTAMP_PUBLIC: u64 = 100000000 + 2;
    const PER_USER_LIMIT: u64 = 123;

    fun setup_test(
        admin: &signer,
        resource_signer: &signer,
        minter_1: &signer,
        aptos_framework: &signer,
        timestamp: u64,
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);
        account::create_account_for_test(signer::address_of(admin));
        account::create_account_for_test(signer::address_of(aptos_framework));
        std::resource_account::create_resource_account(admin, vector<u8> [], vector<u8> []);

        package_manager::init_module_for_test(resource_signer);

        let (burn, mint) = aptos_framework::aptos_coin::initialize_for_test(aptos_framework);
        allowlist::setup_account<AptosCoin>(minter_1, MINTER_STARTING_COINS, &mint);
        coin::destroy_burn_cap(burn);
        coin::destroy_mint_cap(mint);

        init_mint_machine_for_test(admin);
    }

    fun init_mint_machine_for_test(admin: &signer) {
        mint_machine::initialize_mint_machine(
            admin,
            str(COLLECTION_DESCRIPTION),
            MAX_SUPPLY,
            str(COLLECTION_NAME),
            str(COLLECTION_URI),
            MUTABLE_COLLECTION_DESCRIPTION,
            MUTABLE_ROYALTY,
            MUTABLE_URI,
            MUTABLE_TOKEN_DESCRIPTION,
            MUTABLE_TOKEN_NAME,
            MUTABLE_TOKEN_PROPERTIES,
            MUTABLE_TOKEN_URI,
            TOKENS_BURNABLE_BY_CREATOR,
            TOKENS_FREEZABLE_BY_CREATOR,
            ROYALTY_NUMERATOR,
            ROYALTY_DENOMINATOR,
            str(TOKEN_BASE_NAME),
        );
    }

    #[test(admin = @deployer, resource_signer = @no_code_mint, minter_1 = @0xAAAA, aptos_framework = @0x1)]
    fun test_happy_path(
        admin: &signer,
        resource_signer: &signer,
        minter_1: &signer,
        aptos_framework: &signer,
    ) {
        let admin_addr = signer::address_of(admin);
        setup_test(admin, resource_signer, minter_1, aptos_framework, START_TIMESTAMP_PUBLIC + 1);
        mint_machine::upsert_tier(
            admin,
            str(b"public"),
            true, // open to public
            1,
            START_TIMESTAMP_PUBLIC,
            END_TIMESTAMP_PUBLIC,
            PER_USER_LIMIT
        );

        add_test_metadata(admin, MAX_SUPPLY);
        mint_machine::assert_ready_for_launch(admin_addr);

        // collection is ready for launch, enable it!
        mint_machine::enable_minting(admin);

        let minter_1_addr = signer::address_of(minter_1);
        let allowlist_addr = mint_machine::get_creator_addr(admin_addr);
        allowlist::assert_eligible_for_tier(allowlist_addr, minter_1_addr, str(b"public"));

        let aptos_token_objects = mint_machine::mint_for_test(minter_1, admin_addr, MAX_SUPPLY);
        vector::enumerate_ref(&aptos_token_objects, |i, aptos_token_object| {
            assert!(object::is_owner(*aptos_token_object, minter_1_addr), i);
            let token_object = object::convert<AptosToken, Token>(*aptos_token_object);
            assert!(token::name(token_object) == mint_machine::concat_any<u64>(str(TOKEN_BASE_NAME), i), i);
        });

        // destroy the allowlist, only possible if all tokens have been minted
        mint_machine::destroy_allowlist(admin);
        assert!(!allowlist::exists_at(admin_addr), 0);
    }

    fun add_test_metadata(
        admin: &signer,
        n: u64
    ) {
        let uris = vector<String> [];
        let descriptions = vector<String> [];
        let property_keys = vector<vector<String>> [];
        let property_values = vector<vector<vector<u8>>> [];
        let property_types = vector<vector<String>> [];
        let base_token_uri = str(TOKEN_BASE_URI);

        let i: u64 = 0;
        while (i < n) {
            vector::push_back(&mut uris, mint_machine::concat_any<u64>(base_token_uri, i));
            vector::push_back(&mut descriptions, str(TOKEN_DESCRIPTION));
            vector::push_back(&mut property_keys, vector<String> [
                str(b"key 1"),
                str(b"key 2"),
                str(b"key 3"),
                str(b"key 4"),
                str(b"key 5"),
            ]);
            vector::push_back(&mut property_values, vector<vector<u8>> [
                bcs::to_bytes(&str(b"value 1")),
                bcs::to_bytes(&str(b"value 2")),
                bcs::to_bytes(&str(b"value 3")),
                bcs::to_bytes(&9001),
                bcs::to_bytes(&true),
            ]);
            vector::push_back(&mut property_types, vector<String> [
                str(b"0x1::string::String"),
                str(b"0x1::string::String"),
                str(b"0x1::string::String"),
                str(b"u64"),
                str(b"bool"),
            ]);
            i = i + 1;
        };
        mint_machine::add_tokens(
            admin,
            uris,
            descriptions,
            property_keys,
            property_values,
            property_types,
            false
        );
    }
}
