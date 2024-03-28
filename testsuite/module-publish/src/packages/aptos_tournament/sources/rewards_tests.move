#[test_only]
module tournament::rewards_tests {
    use std::account;
    use std::signer;
    use std::string;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::Self;
    use aptos_framework::object::Self;
    use aptos_token::token;

    use tournament::admin;
    use tournament::aptos_tournament;
    use tournament::rewards;
    use tournament::test_utils;
    use tournament::token_manager;
    use tournament::tournament_manager;

    const OCTAS: u64 = 10_000_000;

    const COLLECTION_NAME: vector<u8> = b"test collection";
    const TOKEN_NAME1: vector<u8> = b"test token 1";
    const TOKEN_NAME2: vector<u8> = b"test token 2";

    fun setup_tournament(
        deployer: &signer,
        admin: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
    ): (address, address, address) {
        token_manager::init_module_for_test(deployer);
        aptos_tournament::init_module_for_test(deployer);
        admin::set_admin_signer(deployer, signer::address_of(admin));

        let tournament_address = aptos_tournament::create_new_tournament_returning(deployer);
        aptos_tournament::set_tournament_joinable(admin, tournament_address);
        // Ensure this works
        let _admin2 = admin::get_admin_signer_as_admin(admin);

        let p1_token = tournament_manager::join_tournament_with_return(
            user1_signer,
            tournament_address,
            string::utf8(b"1")
        );
        let p2_token = tournament_manager::join_tournament_with_return(
            user2_signer,
            tournament_address,
            string::utf8(b"2")
        );
        (tournament_address, object::object_address(&p1_token), object::object_address(&p2_token))
    }

    fun setup_addresses(
        aptos_framework: &signer,
        deployer: &signer,
        funds_holder_signer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
    ): (address, signer, token::TokenId, token::TokenId, address, address) {
        test_utils::init_test_framework(aptos_framework, 1);
        test_utils::enable_features_for_test(aptos_framework);

        account::create_account_for_test(signer::address_of(user1_signer));
        account::create_account_for_test(signer::address_of(user2_signer));

        let (tournament_address, p1_token_address, p2_token_address) = setup_tournament(
            deployer,
            funds_holder_signer,
            user1_signer,
            user2_signer,
        );
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(funds_holder_signer, tournament_address);
        assert!(tournament_address == signer::address_of(&tournament_signer), 101);

        let mint = test_utils::get_mint_capabilities(aptos_framework);
        test_utils::fund_account(funds_holder_signer, 10 * OCTAS, &mint);
        coin::destroy_mint_cap(mint);


        let collection_name = string::utf8(COLLECTION_NAME);
        let description = string::utf8(b"description");
        let url = string::utf8(b"https://not really a url dot com");
        let token_name1 = string::utf8(TOKEN_NAME1);
        let token_name2 = string::utf8(TOKEN_NAME2);

        token::create_collection(
            funds_holder_signer,
            collection_name,
            description,
            url,
            0,
            vector[true, true, true],
        );

        let token_mutate_config = token::create_token_mutability_config(&vector[true, true, true, true, true]);

        let token_data_id1 = token::create_tokendata(
            funds_holder_signer,
            collection_name,
            token_name1,
            description,
            0,
            url,
            signer::address_of(funds_holder_signer),
            100,
            1,
            token_mutate_config,
            vector[],
            vector[],
            vector[],
        );
        let token_data_id2 = token::create_tokendata(
            funds_holder_signer,
            collection_name,
            token_name2,
            description,
            0,
            url,
            signer::address_of(funds_holder_signer),
            100,
            1,
            token_mutate_config,
            vector[],
            vector[],
            vector[],
        );

        let token_id1 = token::mint_token(
            funds_holder_signer,
            token_data_id1,
            1
        );
        let token_id2 = token::mint_token(
            funds_holder_signer,
            token_data_id2,
            1
        );

        (tournament_address, tournament_signer, token_id1, token_id2, p1_token_address, p2_token_address)
    }

    #[test(
        aptos_framework = @0x1,
        deployer = @tournament,
        funds_holder_signer = @0xca54,
        user1_signer = @0x111,
        user2_signer = @0x222,
    )]
    fun test_e2e(
        aptos_framework: &signer,
        deployer: &signer,
        funds_holder_signer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
    ) {
        let (
            tournament_address,
            _tournament_signer,
            token_id1,
            token_id2,
            p1_token_address,
            p2_token_address
        ) = setup_addresses(
            aptos_framework,
            deployer,
            funds_holder_signer,
            user1_signer,
            user2_signer,
        );

        // Set up the aptos coin pool
        aptos_tournament::initialize_and_fund_coin_reward_pool<AptosCoin>(
            funds_holder_signer,
            tournament_address,
            3 * OCTAS,
            4 * OCTAS
        );

        // Deposit the coins and the NFT
        rewards::deposit_coin_rewards<AptosCoin>(
            funds_holder_signer,
            tournament_address,
            1 * OCTAS
        );

        // Test adding rewards to an existing pool
        rewards::deposit_coin_rewards<AptosCoin>(
            funds_holder_signer,
            tournament_address,
            3 * OCTAS
        );

        {
            let (creator, collection, name, property_version) = token::get_token_id_fields(&token_id1);
            aptos_tournament::initialize_and_fund_token_pool(
                funds_holder_signer,
                tournament_address,
                vector[creator],
                vector[collection],
                vector[name],
                vector[property_version],
            );
        };

        {
            let (creator, collection, name, property_version) = token::get_token_id_fields(&token_id2);
            rewards::deposit_token_v1_rewards(
                funds_holder_signer,
                tournament_address,
                vector[creator],
                vector[collection],
                vector[name],
                vector[property_version],
            );
        };

        // This is mandatory for the user to get a tokenv1
        token::opt_in_direct_transfer(user1_signer, true);
        token::opt_in_direct_transfer(user2_signer, true);

        aptos_tournament::end_tournament(funds_holder_signer, tournament_address);

        let user1_address = signer::address_of(user1_signer);
        rewards::claim_coin_reward<AptosCoin>(user1_signer, signer::address_of(user1_signer), p1_token_address);
        assert!(coin::balance<AptosCoin>(user1_address) == 3 * OCTAS, 0);

        let user2_address = signer::address_of(user2_signer);
        rewards::claim_coin_reward<AptosCoin>(user2_signer, signer::address_of(user2_signer), p2_token_address);
        assert!(coin::balance<AptosCoin>(user2_address) == 3 * OCTAS, 10);

        // token 2 is pushed on last, so popped off first
        rewards::claim_token_v1_reward(user2_signer, signer::address_of(user2_signer), p2_token_address);
        assert!(token::balance_of(user2_address, token_id2) > 0, 20);

        rewards::claim_token_v1_reward(user1_signer, signer::address_of(user1_signer), p1_token_address);
        assert!(token::balance_of(user1_address, token_id1) > 0, 22);
    }

    // Error: ENOT_TOKEN_OWNER
    #[expected_failure(abort_code = 7, location = rewards)]
    #[test(
        aptos_framework = @0x1,
        deployer = @tournament,
        funds_holder_signer = @0xca54,
        user1_signer = @0x111,
        user2_signer = @0x222,
    )]
    fun test_not_owner(
        aptos_framework: &signer,
        deployer: &signer,
        funds_holder_signer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
    ) {
        let (
            tournament_address,
            tournament_signer,
            _token_id1,
            _token_id2,
            _p1_token_address,
            p2_token_address
        ) = setup_addresses(
            aptos_framework,
            deployer,
            funds_holder_signer,
            user1_signer,
            user2_signer,
        );

        // Set up the aptos coin pool
        rewards::initialize_reward_pool<AptosCoin>(&tournament_signer, 3 * OCTAS);

        // Deposit the coins and the NFT
        rewards::deposit_coin_rewards<AptosCoin>(
            funds_holder_signer,
            tournament_address,
            5 * OCTAS
        );

        aptos_tournament::end_tournament(funds_holder_signer, tournament_address);

        // Try to have user 1 claim user 2's token (this should fail)
        rewards::claim_token_v1_reward(user1_signer, signer::address_of(user1_signer), p2_token_address);

        aptos_tournament::withdraw_rewards<AptosCoin>(
            funds_holder_signer,
            tournament_address,
            signer::address_of(funds_holder_signer)
        );
    }
}
