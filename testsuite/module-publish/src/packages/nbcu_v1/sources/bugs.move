module 0xABCD::bugs {
    use aptos_framework::timestamp;
    use aptos_framework::account;
    use aptos_token::token::{Self, TokenDataId, TokenId};
    use std::bcs;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String, is_empty};
    use std::vector;
    use aptos_token::property_map::{Self, PropertyMap};
    use aptos_std::table::{Self, Table};

    // Errors
    /// The user is not authorized to perform this action
    const ENOT_AUTHORIZED: u64 = 1;
    /// The user has already claimed their mint
    const EHAS_ALREADY_CLAIMED_MINT: u64 = 2;
    /// Minting is not enabled
    const EMINTING_NOT_ENABLED: u64 = 3;
    /// All of the mints have already been claimed
    const EALREADY_MINTED_THEM_ALL: u64 = 4;
    /// User has already claimed their points for the day
    const EALREADY_GOT_QUIZ_POINTS_TODAY: u64 = 5;
    /// User has already claimed their tickets for the week
    const EALREADY_GOT_TICKET_POINTS_THIS_WEEK: u64 = 6;
    /// User has already claimed their referrals for the day
    const EALREADY_GOT_REFERRAL_POINTS_THIS_DAY: u64 = 7;

    /// Uknown point type
    const EUNKNOWN_POINT_TYPE: u64 = 10;

    // const ADMIN_ADDRESS: address = @admin;
    const MAX_QUIZ_POINT_CALLS_PER_DAY: u64 = 2;
    const MAX_TICKET_POINT_CALLS_PER_WEEK: u64 = 10;
    const MAX_REFERRAL_POINT_CALLS_PER_DAY: u64 = 10;

    const POINTS_TYPE_QUIZ: vector<u8> = b"quiz";
    const POINTS_TYPE_TICKET: vector<u8> = b"ticket";
    const POINTS_TYPE_REFERRAL: vector<u8> = b"referral";
    const POINTS_TYPE_OTHER: vector<u8> = b"other";


    // Do set up
    fun init_module(publisher: &signer) {
        // Set up NFT collection
        initialize_collection(publisher);
    }

    /// Mints the token for the user
    /// The admin account is required here to prevent people directly calling this
    /// This does _not_ currently enforce idempotency- if you call this twice, you'll get two tokens
    /// This is because we don't want to store a mapping of users to tokens, as we can do this with redis
    public entry fun mint_token(
        _payer_account: &signer,
        admin_signer: &signer,
        user_signer: &signer,
    ) acquires MinterConfig, MintedTokens {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);
        assert!(is_minting_enabled(signer::address_of(admin_signer)), EMINTING_NOT_ENABLED);
        assert!(!is_mint_limit_reached(signer::address_of(admin_signer)), EALREADY_MINTED_THEM_ALL);
        assert!(!is_token_minted(signer::address_of(user_signer), signer::address_of(admin_signer)), EHAS_ALREADY_CLAIMED_MINT);

        mint_new_token(user_signer, signer::address_of(admin_signer));
    }


    // ----------------
    // from tokens.move:
    // ----------------

    // Holds the SignerCap for the resource account that controls the NFT collection
    struct MinterConfig has key {
        signer_cap: account::SignerCapability,
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

    public(friend) fun initialize_collection(publisher: &signer) {
        // Create the signer capability for the collection
        let (resource_signer, signer_cap) = account::create_resource_account(publisher, vector::empty());

        let collection_name = string::utf8(COLLECTION_NAME);
        let description = string::utf8(COLLECTION_DESCRIPTION);
        let collection_uri = string::utf8(COLLECTION_URL);
        // One million tokens max
        let maximum_supply = 1000000;
        let mutate_setting = vector<bool>[ false, false, false ];

        // Create the Minter resource and publish it under the publisher's address
        move_to(publisher, MinterConfig {
            signer_cap,
            minting_enabled: true,
        });

        // Create the MintedTokens resource and publish it under the publisher's address
        move_to(publisher, MintedTokens {
            minted_tokens: table::new(),
        });

        token::create_collection(
            &resource_signer,
            collection_name,
            description,
            collection_uri,
            maximum_supply,
            mutate_setting
        );
    }

    public(friend) fun get_resource_signer(publisher: address): signer acquires MinterConfig {
        account::create_signer_with_capability(&borrow_global<MinterConfig>(publisher).signer_cap)
    }

    public(friend) fun is_mint_limit_reached(publisher: address): bool acquires MinterConfig {
        let resource_address = signer::address_of(&get_resource_signer(publisher));
        let collection_name = string::utf8(COLLECTION_NAME);

        let maximum_supply = token::get_collection_maximum(resource_address, collection_name);
        let minted_count = option::extract(&mut token::get_collection_supply(resource_address, collection_name));

        minted_count >= maximum_supply
    }

    public(friend) fun is_minting_enabled(publisher: address): bool acquires MinterConfig {
        borrow_global<MinterConfig>(publisher).minting_enabled
    }

    public(friend) fun set_minting_enabled(publisher: address, enabled: bool) acquires MinterConfig {
        borrow_global_mut<MinterConfig>(publisher).minting_enabled = enabled
    }

    public(friend) fun mint_new_token(user: &signer, publisher: address): TokenId acquires MinterConfig, MintedTokens {
        let tokendata_id = create_token_data(publisher, string::utf8(TOKEN_URI));
        // this will create a token with property version zero
        let creator = &get_resource_signer(publisher);
        let token_id = token::mint_token(creator, tokendata_id, 1);

        // This mutation will result in a property version of 1
        let new_points = option::some(0);
        let rarity = option::some(string::utf8(DEFAULT_RARITY));
        let combined_times = option::some(last_timestamp_and_times_to_combined(1, 0));
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

        // Move the token to the user
        let token = token::withdraw_token(creator, token_id, 1);
        token::deposit_token(user, token);

        set_token_minted(signer::address_of(user), publisher);

        token_id
    }

    public(friend) fun is_token_minted(user_address: address, publisher: address): bool acquires MintedTokens {
        let minted_tokens = &borrow_global<MintedTokens>(publisher).minted_tokens;
        table::contains(minted_tokens, user_address)
    }

    public(friend) fun set_token_minted(user_address: address, publisher: address) acquires MintedTokens {
        let minted_tokens = &mut borrow_global_mut<MintedTokens>(publisher).minted_tokens;
        table::add(minted_tokens, user_address, true);
    }

    /// Make the tokendata name unique by appending a ` #` and the index
    fun build_token_name(token_prefix: String, index: u64): String {
        string::append_utf8(&mut token_prefix, b" #");
        string::append(&mut token_prefix, u64_to_string(index));
        token_prefix
    }

    fun create_token_data(
        publisher: address,
        token_uri: String,
    ): TokenDataId acquires MinterConfig {
        // Set up the NFT
        let nft_maximum: u64 = 1;

        let collection_name = string::utf8(COLLECTION_NAME);
        let tokendata_description = string::utf8(TOKEN_DESCRIPTION);

        let token_signer = get_resource_signer(publisher);

        // Make the tokendata name unique by appending a ` #` and the current supply + 1
        let current_supply_opt = token::get_collection_supply(signer::address_of(&token_signer), collection_name);
        let index = option::extract(&mut current_supply_opt) + 1;
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
            &token_signer,
            collection_name,
            tokendata_name,
            tokendata_description,
            nft_maximum,
            token_uri,
            signer::address_of(&token_signer),
            ROYALTY_POINTS_DENOMINATOR,
            ROYALTY_POINTS_NUMERATOR,
            token_mutate_config,
            property_keys,
            property_values,
            property_types
        )
    }

    /// This mutates token properties after the token has been minted
    /// This is because we assume that the property_version is 1 after the mint process
    public(friend) fun mutate_token_properties(
        publisher: address,
        token_owner: address,
        token_name: String,
        new_points: Option<u64>,
        new_times_combined_quiz: Option<u64>,
        new_times_combined_tickets: Option<u64>,
        new_times_combined_referrals: Option<u64>,
        rarity: Option<String>
    ) acquires MinterConfig {
        let tokendata_id = get_tokendata_id(publisher, token_name);
        let token_id = token::create_token_id(tokendata_id, 1);

        let (keys, values, types) = make_property_map(
            new_points,
            new_times_combined_quiz,
            new_times_combined_tickets,
            new_times_combined_referrals,
            rarity
        );

        token::mutate_one_token(
            &get_resource_signer(publisher),
            token_owner,
            token_id,
            keys,
            values,
            types,
        );
    }

    /// Mutates the tokendata url given the token name
    public(friend) fun mutate_token_uri(publisher: address, token_name: String, new_token_uri: String) acquires MinterConfig {
        let tokendata_id = get_tokendata_id(publisher, token_name);
        let token_signer = get_resource_signer(publisher);
        token::mutate_tokendata_uri(&token_signer, tokendata_id, new_token_uri);
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


    fun get_tokendata_id(publisher: address, token_name: String): TokenDataId acquires MinterConfig {
        let creator = signer::address_of(&get_resource_signer(publisher));
        let collection_name = string::utf8(COLLECTION_NAME);
        let token_data_id = token::create_token_data_id(
            creator,
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

    #[test(publisher = @0xABCD, user1 = @0x7001, user2 = @0x7002)]
    fun test_minting(
        publisher: signer,
        user1: signer,
        user2: signer,
    ) acquires MinterConfig, MintedTokens {
        use aptos_framework::account;

        account::create_account_for_test(signer::address_of(&publisher));
        account::create_account_for_test(signer::address_of(&user1));
        account::create_account_for_test(signer::address_of(&user2));

        initialize_collection(&publisher);

        let token_id1 = mint_new_token(&user1, signer::address_of(&publisher));
        let (creator, collection, name) = token::get_token_data_id_fields(&token::get_tokendata_id(signer::address_of(&publisher), token_id1));
        assert!(creator == signer::address_of(&get_resource_signer(signer::address_of(&publisher))), 1);
        assert!(collection == string::utf8(COLLECTION_NAME), 2);
        assert!(name == build_token_name(string::utf8(TOKEN_NAME), 1), 3);

        let token_id2 = mint_new_token(&user2, signer::address_of(&publisher));
        let (creator, collection, name) = token::get_token_data_id_fields(&token::get_tokendata_id(signer::address_of(&publisher), token_id2));
        assert!(creator == signer::address_of(&get_resource_signer(signer::address_of(&publisher))), 11);
        assert!(collection == string::utf8(COLLECTION_NAME), 12);
        assert!(name == build_token_name(string::utf8(TOKEN_NAME), 2), 13);
    }


    // ----------------
    // from utils.move:
    // ----------------

    // EST offset in seconds from UTC
    // 5 for EST, 4 for EDT
    const EST_OFFSET: u64 = 4 * 60 * 60;
    // number of seconds per day
    const SECONDS_PER_DAY: u64 = 60 * 60 * 24;

    // checks whether two Unix timestamps fall on the same day in EST using midnight as the cutoff
    public fun is_same_day_in_est_midnight(timestamp1: u64, timestamp2: u64): bool {
        // In the event we have an invalid small timestamp, this is the initial condition, so we return false
        if (timestamp1 < 100 || timestamp2 < 100) {
            return false
        };

        // Calculate the number of seconds since midnight EST for each timestamp
        let seconds1 = (timestamp1 - EST_OFFSET) / SECONDS_PER_DAY;
        let seconds2 = (timestamp2 - EST_OFFSET) / SECONDS_PER_DAY;

        // If the seconds since noon are the same, the timestamps are on the same day
        seconds1 == seconds2
    }

    // Legacy backwards compat
    public fun is_same_day_in_pst_noon(_timestamp1: u64, _timestamp2: u64): bool {
        return false
    }

    // checks whether two Unix timestamps are less than a week apart
    public fun is_within_one_week(timestamp1: u64, timestamp2: u64): bool {
        // In the event we have an invalid small timestamp, this is the initial condition, so we return false
        if (timestamp1 < 100 || timestamp2 < 100) {
            return false
        };
        // Ensures timestamp1 <= timestamp2
        if (timestamp1 > timestamp2) {
            let temp = timestamp1;
            timestamp1 = timestamp2;
            timestamp2 = temp;
        };

        (timestamp2 - timestamp1) < SECONDS_PER_DAY * 7
    }

    // Splits a combined timestamp and # times called into a tuple of the timestamp and times called
    public fun combined_to_last_timestamp_and_times(combined: u64): (u64, u64) {
        let last_timestamp = combined / 10;
        let times = combined % 10;
        (last_timestamp, times)
    }

    // Combines a timestamp and # times called into a single u64
    public fun last_timestamp_and_times_to_combined(last_timestamp: u64, times: u64): u64 {
        last_timestamp * 10 + times
    }

    public fun u64_to_string(value: u64): string::String {
        if (value == 0) {
            return string::utf8(b"0")
        };
        let buffer = vector::empty<u8>();
        while (value != 0) {
            vector::push_back(&mut buffer, ((48 + value % 10) as u8));
            value = value / 10;
        };
        vector::reverse(&mut buffer);
        string::utf8(buffer)
    }

    #[test]
    fun test_is_same_day_in_est_midnight() {
        // This is only during DST. If we are not in DST, we need to change this to 0
        let offset_delta_sec =  60 * 60;
        // 1677299008: Fri Feb 24 2023 23:23:28
        // 1677302555: Sat Feb 25 2023 00:22:35
        assert!(!is_same_day_in_est_midnight(1677299008 - offset_delta_sec, 1677302555 - offset_delta_sec), 0);

        // 1678280400: Wed Mar 08 2023 8 AM EST
        // 1678334400: Wed Mar 08 2023 11 PM EST
        assert!(is_same_day_in_est_midnight(1678280400 - offset_delta_sec, 1678334400 - offset_delta_sec), 1);

        // Test starting conditions; i.e invalid start times
        assert!(!is_same_day_in_est_midnight(10, 1677259355 - offset_delta_sec), 2);
    }

    #[test]
    fun test_is_within_one_week() {
        // 1675497600: Sat Feb 04 2023 00:00:00 GMT-0800
        // 1676052000: Fri Feb 10 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675497600, 1676052000), 0);

        // 1675706400: Mon Feb 06 2023 18:00:00 GMT+0000
        // 1676052000: Fri Feb 10 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675706400, 1676052000), 1);

        // 1675706400: Mon Feb 06 2023 18:00:00 GMT+0000
        // 1675965600: Thu Feb 09 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675706400, 1675965600), 2);

        // 1675792800: Tue Feb 07 2023 18:00:00 GMT+0000
        // 1676224800: Sun Feb 12 2023 10:00:00 GMT-0800
        assert!(is_within_one_week(1675792800, 1676224800), 3);

        // 1676224800: Sun Feb 12 2023 10:00:00 GMT-0800
        // 1676233800: Sun Feb 12 2023 12:30:00 GMT-0800
        assert!(is_within_one_week(1676224800, 1676233800), 4);

        // 1676340000: Mon Feb 13 2023 18:00:00 GMT-0800
        // 1676833200: Sun Feb 19 2023 11:00:00 GMT-0800
        assert!(is_within_one_week(1676340000, 1676833200), 5);

        // 1675497600: Sat Feb 04 2023 00:00:00 GMT-0800
        // 1676224800: Sun Feb 12 2023 10:00:00 GMT-0800
        assert!(!is_within_one_week(1675497600, 1676224800), 6);

        // Test starting conditions; i.e invalid start times
        assert!(!is_within_one_week(10, 1676224800), 6);
    }


    #[test]
    fun test_combined_times() {
        // 1677302555: 9:22 PM EST
        let timestamp = 1677302555;
        let times = 3;
        let combined = last_timestamp_and_times_to_combined(timestamp, times);
        let (last_timestamp, last_times) = combined_to_last_timestamp_and_times(combined);
        assert!(last_timestamp == timestamp, 0);
        assert!(last_times == times, 1);
    }


    #[test]
    fun test_u64_to_string() {
        let test_cases: vector<u64> = vector[0, 1, 10, 100, 1000, 987654321, 1000000];
        let expected: vector<string::String> = vector[
            string::utf8(b"0"),
            string::utf8(b"1"),
            string::utf8(b"10"),
            string::utf8(b"100"),
            string::utf8(b"1000"),
            string::utf8(b"987654321"),
            string::utf8(b"1000000"),
        ];
        while (vector::length(&test_cases) > 0) {
            let test_case = vector::pop_back(&mut test_cases);
            let expected = vector::pop_back(&mut expected);
            assert!(u64_to_string(test_case) == expected, test_case);
        };
    }
}
