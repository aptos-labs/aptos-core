module self::bugs {

    use aptos_framework::timestamp;
    use self::tokens;
    use std::option;
    use std::signer;
    use std::string::{Self, String, is_empty};
    use self::utils;
    use self::parallel_vector;
    use aptos_token::token::{Self, Token};
    use std::bcs;
    use std::vector;

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
    /// Minting must be disabled when filling the parallel vector
    const EMINTING_MUST_BE_DISABLED: u64 = 8;

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

    const PV_PARALLELISM: u64 = 16;
    const PV_BUCKET_SIZE: u64 = 16;

    // Do set up
    fun init_module(publisher: &signer) {
        // Set up NFT collection
        tokens::initialize_collection(publisher);

        // set up the parallel vector
        parallel_vector::create<Token>(publisher, PV_PARALLELISM, PV_BUCKET_SIZE);
    }

    public entry fun initialize_parallel_vector(
        _payer_account: &signer,
        admin_signer: &signer,
    ) {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);
        parallel_vector::create<Token>(admin_signer, PV_PARALLELISM, PV_BUCKET_SIZE);
    }

    public entry fun fill_parallel_vector(admin_signer: &signer, number_to_fill: u64) {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);
        // assert!(!tokens::is_minting_enabled(), EMINTING_MUST_BE_DISABLED);
        let i = 0;
        let pv_address = signer::address_of(admin_signer);
        let start_offset = parallel_vector::length<Token>(pv_address);
        while (i < number_to_fill) {
            i = i + 1;
            let index = 1000000 - (start_offset + i);
            let token = tokens::mint_new_token_with_index(admin_signer, index);
            parallel_vector::push_back<Token>(pv_address, token, index);
        }
    }

    /// Mints the token for the user
    /// The admin account is required here to prevent people directly calling this
    /// This does _not_ currently enforce idempotency- if you call this twice, you'll get two tokens
    /// This is because we don't want to store a mapping of users to tokens, as we can do this with redis
    public entry fun mint_token(
        _payer_account: &signer,
        admin_signer: &signer,
        user_signer: &signer,
    ) {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);
        // assert!(tokens::is_minting_enabled(), EMINTING_NOT_ENABLED);
        assert!(!tokens::is_mint_limit_reached(admin_signer), EALREADY_MINTED_THEM_ALL);

        let user_address = signer::address_of(user_signer);
        assert!(!tokens::is_token_minted(admin_signer, user_address), EHAS_ALREADY_CLAIMED_MINT);

        let user_address_bytes = bcs::to_bytes(&user_address);
        let index = *vector::borrow(&user_address_bytes, 0);
        let token = parallel_vector::pop_back_index<Token>(
            signer::address_of(admin_signer),
            (index as u64)
        );
        token::deposit_token(user_signer, token);
        tokens::set_token_minted(admin_signer, user_address);
    }

    /// Sets minting to be enabled or disabled
    public entry fun set_minting_enabled(
        _payer_account: &signer,
        admin_signer: &signer,
        enabled: bool,
    ) {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);
        tokens::set_minting_enabled(admin_signer, enabled);
    }

    /// This adds points to a users NFT. This involves:
    /// 1. Getting the NFT
    /// 2. Adding points to it's points property
    /// The limits:
    ///     - Max 2 quizzes per day
    ///     - Max 10 tickets per week
    ///     - Max 10 referrals per day
    public entry fun add_points_to_user(
        _payer_account: &signer,
        admin_signer: &signer,
        token_owner: address,
        token_name: String,
        earned_points: u64,
        type: String,
    ) {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);

        let new_combined_times_quiz: u64 = 0;
        let new_combined_times_ticket: u64 = 0;
        let new_combined_times_referral: u64 = 0;

        if (type == string::utf8(POINTS_TYPE_QUIZ)) {
            let current_combined_times = tokens::get_combined_quiz_timestamp_and_times_called(admin_signer, token_owner, token_name);
            let (last_timestamp, times_called) = utils::combined_to_last_timestamp_and_times(current_combined_times);
            // Make sure we haven't called it twice today yet, or that we're on a new day
            let current_timestamp = timestamp::now_seconds();
            let new_times_called = if (utils::is_same_day_in_est_midnight(last_timestamp, current_timestamp)) {
                // If we're calling it within the same day, make sure we haven't called it too many times
                assert!(times_called < MAX_QUIZ_POINT_CALLS_PER_DAY, EALREADY_GOT_QUIZ_POINTS_TODAY);
                times_called + 1
            } else {
                // If we're calling it on a new day, reset the times called to 1
                1
            };
            new_combined_times_quiz = utils::last_timestamp_and_times_to_combined(current_timestamp, new_times_called);
        } else if (type == string::utf8(POINTS_TYPE_TICKET)) {
            let current_combined_times = tokens::get_combined_tickets_timestamp_and_times_called(
                admin_signer,
                token_owner,
                token_name
            );
            let (last_timestamp, times_called) = utils::combined_to_last_timestamp_and_times(current_combined_times);
            // Make sure we haven't called it 10 times this week yet, or that we're on a new week
            let current_timestamp = timestamp::now_seconds();
            if (utils::is_within_one_week(last_timestamp, current_timestamp)) {
                // If we're calling it within the same week, make sure we haven't called it too many times
                // Don't reset the week yet, as we'll do that once we're > 7 days out
                assert!(times_called < MAX_TICKET_POINT_CALLS_PER_WEEK, EALREADY_GOT_TICKET_POINTS_THIS_WEEK);
                new_combined_times_ticket = utils::last_timestamp_and_times_to_combined(
                    last_timestamp,
                    times_called + 1
                );
            } else {
                // If we're calling it on a new week, reset the times called to 1
                new_combined_times_ticket = utils::last_timestamp_and_times_to_combined(
                    current_timestamp,
                    1
                );
            };
        } else if (type == string::utf8(POINTS_TYPE_REFERRAL)) {
            let current_combined_times = tokens::get_combined_referral_timestamp_and_times_called(
                admin_signer,
                token_owner,
                token_name
            );
            let (last_timestamp, times_called) = utils::combined_to_last_timestamp_and_times(current_combined_times);
            // Make sure we haven't called it 10 times this day yet, or that we're on a new day
            let current_timestamp = timestamp::now_seconds();
            let new_times_called = if (utils::is_same_day_in_est_midnight(last_timestamp, current_timestamp)) {
                // If we're calling it within the same day, make sure we haven't called it too many times
                assert!(times_called < MAX_REFERRAL_POINT_CALLS_PER_DAY, EALREADY_GOT_REFERRAL_POINTS_THIS_DAY);
                times_called + 1
            } else {
                // If we're calling it on a new day, reset the times called to 1
                1
            };
            new_combined_times_referral = utils::last_timestamp_and_times_to_combined(
                current_timestamp,
                new_times_called
            );
        }  else if (type == string::utf8(POINTS_TYPE_OTHER)) {
            // Do nothing- there is no time limit for this one
        } else {
            abort EUNKNOWN_POINT_TYPE
        };

        let current_token_points = tokens::get_token_points(admin_signer, token_owner, token_name);
        let new_points = current_token_points + earned_points;
        update_token_properties_inner(
            admin_signer,
            token_owner,
            token_name,
            new_points,
            new_combined_times_quiz,
            new_combined_times_ticket,
            new_combined_times_referral,
            string::utf8(b"")
        );
    }

    /// This allows the admin to update the rarity + level of a token
    /// This is called after the nightly cron job raffle
    /// If the `String` is empty, it will not be updated
    public entry fun update_token_rarity(
        _payer_account: &signer,
        admin_signer: &signer,
        token_owner: address,
        token_name: String,
        new_rarity: String,
        new_token_uri: String,
    ) {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);
        if (!string::is_empty(&new_rarity)) {
            update_token_properties_inner(admin_signer, token_owner, token_name, 0, 0, 0, 0, new_rarity);
        };
        if (!string::is_empty(&new_token_uri)) {
            tokens::mutate_token_uri(admin_signer, token_name, new_token_uri);
        };
    }

    /// This allows the admin to arbitrarily update the properties of a token
    /// This is useful for testing, but also for fixing bugs, changing scaling, etc
    /// If the u64 is 0, or the `String` is empty, it will not be updated
    public entry fun update_token_properties(
        _payer_account: &signer,
        admin_signer: &signer,
        token_owner: address,
        token_name: String,
        new_points: u64,
        new_times_combined_quiz: u64,
        new_times_combined_tickets: u64,
        new_times_combined_referral: u64,
        rarity: String,
    ) {
        // assert!(signer::address_of(admin_signer) == ADMIN_ADDRESS, ENOT_AUTHORIZED);
        update_token_properties_inner(
            admin_signer,
            token_owner,
            token_name,
            new_points,
            new_times_combined_quiz,
            new_times_combined_tickets,
            new_times_combined_referral,
            rarity
        );
    }

    fun update_token_properties_inner(
        creator: &signer,
        token_owner: address,
        token_name: String,
        new_points: u64,
        new_times_combined_quiz: u64,
        new_times_combined_tickets: u64,
        new_times_combined_referral: u64,
        rarity: String
    ) {
        let new_points_opt = option::none<u64>();
        if (new_points != 0) {
            new_points_opt = option::some(new_points);
        };

        let new_times_quiz_opt = option::none<u64>();
        if (new_times_combined_quiz != 0) {
            new_times_quiz_opt = option::some(new_times_combined_quiz);
        };

        let new_times_tickets_opt = option::none<u64>();
        if (new_times_combined_tickets != 0) {
            new_times_tickets_opt = option::some(new_times_combined_tickets);
        };

        let new_times_referral_opt = option::none<u64>();
        if (new_times_combined_referral != 0) {
            new_times_referral_opt = option::some(new_times_combined_referral);
        };

        let rarity_opt = option::none<String>();
        if (!is_empty(&rarity)) {
            rarity_opt = option::some(rarity);
        };

        tokens::mutate_token_properties(
            creator,
            token_owner,
            token_name,
            new_points_opt,
            new_times_quiz_opt,
            new_times_tickets_opt,
            new_times_referral_opt,
            rarity_opt,
        );
    }


    #[test(publisher = @self, framework = @0x01, fee_payer = @0x5001, user1 = @0x7001, user2 = @0x7002)]
    fun test_minting(
        publisher: signer,
        framework: signer,
        admin: signer,
        user1: signer,
        user2: signer,
    ) {
        use aptos_framework::account;


        timestamp::set_time_has_started_for_testing(&framework);

        account::create_account_for_test(@self);
        account::create_account_for_test(signer::address_of(&user1));
        account::create_account_for_test(signer::address_of(&user2));

        init_module(&publisher);

        tokens::set_minting_enabled(false);
        fill_parallel_vector(&publisher, 100);
        tokens::set_minting_enabled(true);

        mint_token(&fee_payer, &publisher, &user1);
        mint_token(&fee_payer, &publisher, &user2);

        add_points_to_user(
            &fee_payer,
            &publisher,
            signer::address_of(&user1),
            100,
            string::utf8(POINTS_TYPE_QUIZ)
        );
    }
}
