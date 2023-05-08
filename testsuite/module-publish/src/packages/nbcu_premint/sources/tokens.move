module self::tokens {
    friend self::bugs;

    use aptos_token::token::{Self, TokenDataId, TokenId, Token};
    use self::utils;
    use std::bcs;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_token::property_map::{Self, PropertyMap};
    use aptos_std::table::{Self, Table};

    // Holds the SignerCap for the resource account that controls the NFT collection
    struct MinterConfig has key {
        minting_enabled: bool,
    }

    // Holds the table that tracks which users have minted a token
    struct MintedTokens has key {
        minted_tokens: Table<address, bool>
    }

    // Token Configuration
    const COLLECTION_NAME: vector<u8> = b"The Renfield Collection";
    const COLLECTION_URL: vector<u8> = b"";
    const COLLECTION_DESCRIPTION: vector<u8> = b"The Renfield NFT collection is a series of four collectible NFTs following the story of Renfield, Dracula's familiar on his quest to free himself from the clutches of the Prince of Darkness. This series features special poster designs and custom comic art based on key scenes from the Universal Pictures film Renfield. Special NFTs are tied to different levels of physical and digital prizes won by playing the \"The Renfield Sweepstakes\", presented by Fandango and brought to life by Aptos";
    const TOKEN_NAME: vector<u8> = b"Renfield Collectible";
    const TOKEN_URI: vector<u8> = b"https://dnjj3np4qtts5.cloudfront.net/movie-poster.png";
    const TOKEN_DESCRIPTION: vector<u8> = b"";

    /* Token URLS and Names:
    Common: https://dnjj3np4qtts5.cloudfront.net/movie-poster.png
    Rare: https://dnjj3np4qtts5.cloudfront.net/heismyservant-comic.png
    Ultra-rare: https://dnjj3np4qtts5.cloudfront.net/collectible-poster.gif
    Legendary https://dnjj3np4qtts5.cloudfront.net/iamdracula-comic.png
    */

    // Property Keys
    const PROPERTY_KEY_POINTS: vector<u8> = b"points";
    const PROPERTY_KEY_RARITY: vector<u8> = b"rarity";
    // Quiz
    const PROPERTY_KEY_QUIZ: vector<u8> = b"q";
    // Ticket Redemptions
    const PROPERTY_KEY_TICKETS: vector<u8> = b"t";
    // Friend Referrals
    const PROPERTY_KEY_REFERRALS: vector<u8> = b"r";

    // Rarity Configs
    const DEFAULT_RARITY: vector<u8> = b"common";

    // Royalty Config (0/100 = 0%)
    const ROYALTY_POINTS_NUMERATOR: u64 = 0;
    const ROYALTY_POINTS_DENOMINATOR: u64 = 100;

    public(friend) fun initialize_collection(creator: &signer) {
        let collection_name = string::utf8(COLLECTION_NAME);
        let description = string::utf8(COLLECTION_DESCRIPTION);
        let collection_uri = string::utf8(COLLECTION_URL);
        // One million tokens max
        let maximum_supply = 1000000;
        let mutate_setting = vector<bool>[ false, false, false ];

        // Create the Minter resource and publish it under the creator's address
        move_to(creator, MinterConfig {
            minting_enabled: true,
        });

        // Create the MintedTokens resource and publish it under the creator's address
        move_to(creator, MintedTokens {
            minted_tokens: table::new(),
        });

        token::create_collection(
            creator,
            collection_name,
            description,
            collection_uri,
            maximum_supply,
            mutate_setting
        );
    }

    public(friend) fun is_mint_limit_reached(creator: &signer): bool {
        let resource_address = signer::address_of(creator);
        let collection_name = string::utf8(COLLECTION_NAME);

        let maximum_supply = token::get_collection_maximum(resource_address, collection_name);
        let minted_count = option::extract(&mut token::get_collection_supply(resource_address, collection_name));

        minted_count >= maximum_supply
    }

    public(friend) fun is_minting_enabled(creator: &signer): bool acquires MinterConfig {
        borrow_global<MinterConfig>(signer::address_of(creator)).minting_enabled
    }

    public(friend) fun set_minting_enabled(creator: &signer, enabled: bool) acquires MinterConfig {
        borrow_global_mut<MinterConfig>(signer::address_of(creator)).minting_enabled = enabled
    }

    public(friend) fun mint_new_token(creator: &signer, user: &signer): TokenId acquires MintedTokens {
        let current_supply_opt = token::get_collection_supply(
            signer::address_of(creator),
            string::utf8(COLLECTION_NAME)
        );
        let index = option::extract(&mut current_supply_opt) + 1;

        // Move the token to the user
        let token = mint_new_token_with_index(creator, index);
        let token_id = token::get_token_id(&token);
        token::deposit_token(user, token);

        set_token_minted(creator, signer::address_of(user));

        token_id
    }

    public(friend) fun mint_new_token_with_index(
        creator: &signer,
        index: u64
    ): Token {
        let tokendata_id = create_token_data_with_index(creator, string::utf8(TOKEN_URI), index);
        // this will create a token with property version zero
        let token_id = token::mint_token(creator, tokendata_id, 1);

        // This mutation will result in a property version of 1
        let new_points = option::some(0);
        let rarity = option::some(string::utf8(DEFAULT_RARITY));
        let combined_times = option::some(utils::last_timestamp_and_times_to_combined(1, 0));
        let (keys, values, types) = make_property_map(
            new_points,
            combined_times,
            combined_times,
            combined_times,
            rarity
        );

        token_id = token::mutate_one_token(
            creator,
            signer::address_of(creator),
            token_id,
            keys,
            values,
            types,
        );

        token::withdraw_token(creator, token_id, 1)
    }

    public(friend) fun is_token_minted(creator: &signer, user_address: address): bool acquires MintedTokens {
        let minted_tokens = &borrow_global<MintedTokens>(signer::address_of(creator)).minted_tokens;
        table::contains(minted_tokens, user_address)
    }

    public(friend) fun set_token_minted(creator: &signer, user_address: address) acquires MintedTokens {
        let minted_tokens = &mut borrow_global_mut<MintedTokens>(signer::address_of(creator)).minted_tokens;
        table::add(minted_tokens, user_address, true);
    }

    public(friend) fun get_token_points(creator: &signer, owner_address: address, token_name: String): u64 {
        let property_map = get_token_properties(creator, owner_address, token_name);
        property_map::read_u64(&property_map, &string::utf8(PROPERTY_KEY_POINTS))
    }

    public(friend) fun get_combined_quiz_timestamp_and_times_called(
        creator: &signer,
        owner_address: address,
        token_name: String
    ): u64 {
        get_combined_timestamp_and_times_called(creator, owner_address, token_name, &string::utf8(PROPERTY_KEY_QUIZ))
    }

    public(friend) fun get_combined_tickets_timestamp_and_times_called(
        creator: &signer,
        owner_address: address,
        token_name: String
    ): u64 {
        get_combined_timestamp_and_times_called(creator, owner_address, token_name, &string::utf8(PROPERTY_KEY_TICKETS))
    }

    public(friend) fun get_combined_referral_timestamp_and_times_called(
        creator: &signer,
        owner_address: address,
        token_name: String
    ): u64 {
        get_combined_timestamp_and_times_called(creator, owner_address, token_name, &string::utf8(PROPERTY_KEY_REFERRALS))
    }

    fun get_combined_timestamp_and_times_called(
        creator: &signer,
        owner_address: address,
        token_name: String,
        property_key: &String
    ): u64 {
        let property_map = get_token_properties(creator, owner_address, token_name);
        property_map::read_u64(&property_map, property_key)
    }

    public(friend) fun get_token_properties(
        creator: &signer,
        owner_address: address,
        token_name: String
    ): PropertyMap {
        let tokendata_id = get_tokendata_id(creator, token_name);
        let token_id = token::create_token_id(tokendata_id, 1);
        token::get_property_map(owner_address, token_id)
    }

    /// Make the tokendata name unique by appending a ` #` and the index
    fun build_token_name(token_prefix: String, index: u64): String {
        string::append_utf8(&mut token_prefix, b" #");
        string::append(&mut token_prefix, utils::u64_to_string(index));
        token_prefix
    }

    fun create_token_data_with_index(
        creator: &signer,
        token_uri: String,
        index: u64
    ): TokenDataId {
        // Set up the NFT
        let nft_maximum: u64 = 1;

        let collection_name = string::utf8(COLLECTION_NAME);
        let tokendata_description = string::utf8(TOKEN_DESCRIPTION);

        let tokendata_name = build_token_name(string::utf8(TOKEN_NAME), index);

        // tokan max mutable: true
        // token URI mutable: true
        // token description mutable: true
        // token royalty mutable: true
        // token properties mutable: true
        let token_mutate_config = token::create_token_mutability_config(&vector<bool>[ true, true, true, true, true ]);

        let property_keys: vector<String> = vector[];
        let property_values: vector<vector<u8>> = vector[];
        let property_types: vector<String> = vector[];

        token::create_tokendata(
            creator,
            collection_name,
            tokendata_name,
            tokendata_description,
            nft_maximum,
            token_uri,
            signer::address_of(creator),
            ROYALTY_POINTS_DENOMINATOR,
            ROYALTY_POINTS_NUMERATOR,
            token_mutate_config,
            property_keys,
            property_values,
            property_types
        )
    }

    fun create_token_data(
        creator: &signer,
        token_uri: String,
    ): TokenDataId {
        let collection_name = string::utf8(COLLECTION_NAME);

        // Make the tokendata name unique by appending a ` #` and the current supply + 1
        let current_supply_opt = token::get_collection_supply(signer::address_of(creator), collection_name);
        let index = option::extract(&mut current_supply_opt) + 1;

        create_token_data_with_index(
            creator,
            token_uri,
            index
        )
    }

    /// This mutates token properties after the token has been minted
    /// This is because we assume that the property_version is 1 after the mint process
    public(friend) fun mutate_token_properties(
        creator: &signer,
        token_owner: address,
        token_name: String,
        new_points: Option<u64>,
        new_times_combined_quiz: Option<u64>,
        new_times_combined_tickets: Option<u64>,
        new_times_combined_referrals: Option<u64>,
        rarity: Option<String>
    ) {
        let tokendata_id = get_tokendata_id(creator, token_name);
        let token_id = token::create_token_id(tokendata_id, 1);

        let (keys, values, types) = make_property_map(
            new_points,
            new_times_combined_quiz,
            new_times_combined_tickets,
            new_times_combined_referrals,
            rarity
        );

        token::mutate_one_token(
            creator,
            token_owner,
            token_id,
            keys,
            values,
            types,
        );
    }

    /// Mutates the tokendata url given the token name
    public(friend) fun mutate_token_uri(creator: &signer, token_name: String, new_token_uri: String) {
        let tokendata_id = get_tokendata_id(creator, token_name);
        token::mutate_tokendata_uri(creator, tokendata_id, new_token_uri);
    }

    fun make_property_map(
        new_points: Option<u64>,
        new_times_combined_quiz: Option<u64>,
        new_times_combined_tickets: Option<u64>,
        new_times_combined_referrals: Option<u64>,
        rarity: Option<String>
    ): (vector<String>, vector<vector<u8>>, vector<String>) {
        let property_keys: vector<String> = vector[];
        let property_values: vector<vector<u8>> = vector[];
        let property_types: vector<String> = vector[];

        if (option::is_some(&new_times_combined_quiz)) {
            let new_times_combined = option::extract(&mut new_times_combined_quiz);
            vector::push_back(&mut property_keys, string::utf8(PROPERTY_KEY_QUIZ));
            vector::push_back(&mut property_values, bcs::to_bytes(&new_times_combined));
            vector::push_back(&mut property_types, string::utf8(b"u64"));
        };

        if (option::is_some(&new_times_combined_tickets)) {
            let new_times_combined = option::extract(&mut new_times_combined_tickets);
            vector::push_back(&mut property_keys, string::utf8(PROPERTY_KEY_TICKETS));
            vector::push_back(&mut property_values, bcs::to_bytes(&new_times_combined));
            vector::push_back(&mut property_types, string::utf8(b"u64"));
        };

        if (option::is_some(&new_times_combined_referrals)) {
            let new_times_combined = option::extract(&mut new_times_combined_referrals);
            vector::push_back(&mut property_keys, string::utf8(PROPERTY_KEY_REFERRALS));
            vector::push_back(&mut property_values, bcs::to_bytes(&new_times_combined));
            vector::push_back(&mut property_types, string::utf8(b"u64"));
        };

        if (option::is_some(&new_points)) {
            let new_points = option::extract(&mut new_points);
            vector::push_back(&mut property_keys, string::utf8(PROPERTY_KEY_POINTS));
            vector::push_back(&mut property_values, bcs::to_bytes(&new_points));
            vector::push_back(&mut property_types, string::utf8(b"u64"));
        };

        if (option::is_some(&rarity)) {
            let rarity = option::extract(&mut rarity);
            vector::push_back(&mut property_keys, string::utf8(PROPERTY_KEY_RARITY));
            vector::push_back(&mut property_values, bcs::to_bytes(&rarity));
            vector::push_back(&mut property_types, string::utf8(b"0x1::string::String"));
        };

        (property_keys, property_values, property_types)
    }

    fun get_tokendata_id(creator: &signer, token_name: String): TokenDataId {
        let collection_name = string::utf8(COLLECTION_NAME);
        let token_data_id = token::create_token_data_id(
            signer::address_of(creator),
            collection_name,
            token_name,
        );
        token_data_id
    }

    #[test]
    fun test_build_token_name() {
        let token_prefix = string::utf8(b"TokenName");
        assert!(build_token_name(token_prefix, 1) == string::utf8(b"TokenName #1"), 1);
        assert!(build_token_name(token_prefix, 987654321) == string::utf8(b"TokenName #987654321"), 10);
    }

    #[test(publisher = @self, user1 = @0x7001, user2 = @0x7002)]
    fun test_minting(
        publisher: signer,
        user1: signer,
        user2: signer,
    ) acquires MinterConfig, MintedTokens {
        use aptos_framework::account;

        account::create_account_for_test(@self);
        account::create_account_for_test(signer::address_of(&user1));
        account::create_account_for_test(signer::address_of(&user2));

        initialize_collection(&publisher);

        let token_id1 = mint_new_token(&publisher, &user1);
        let (creator, collection, name) = token::get_token_data_id_fields(&token::get_tokendata_id(&publisher, token_id1));
        assert!(creator == signer::address_of(&publisher), 1);
        assert!(collection == string::utf8(COLLECTION_NAME), 2);
        assert!(name == build_token_name(string::utf8(TOKEN_NAME), 1), 3);

        let token_id2 = mint_new_token(&publisher, &user2);
        let (creator, collection, name) = token::get_token_data_id_fields(&token::get_tokendata_id(&publisher, token_id2));
        assert!(creator == signer::address_of(&publisher), 11);
        assert!(collection == string::utf8(COLLECTION_NAME), 12);
        assert!(name == build_token_name(string::utf8(TOKEN_NAME), 2), 13);
    }
}
