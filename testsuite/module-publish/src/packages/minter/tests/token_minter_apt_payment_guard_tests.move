#[test_only]
module token_minter::token_minter_apt_payment_guard_tests {
    use std::bcs;
    use std::signer;
    use std::string;
    use std::string::String;
    use aptos_framework::aptos_coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;

    use token_minter::apt_payment;
    use token_minter::coin_utils;
    use token_minter::token_minter;
    use token_minter::token_minter_utils;

    fun setup_test_environment(
        fx: &signer,
        user: &signer,
        creator: &signer,
        user_initial_balance: u64,
        creator_initial_balance: u64,
    ) {
        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(fx);
        coin_utils::fund_account(&mint_cap, user, user_initial_balance);
        coin_utils::fund_account(&mint_cap, creator, creator_initial_balance);
        coin_utils::clean_up_caps(burn_cap, mint_cap);
    }

    #[test(creator = @0x123, user = @0x456, fx = @0x1)]
    fun test_add_apt_payment_guard_and_mint(creator: &signer, user: &signer, fx: &signer) {
        let apt_cost = 50;
        let user_initial_balance = 100;
        let creator_initial_balance = 0;
        let destination = signer::address_of(creator);
        setup_test_environment(fx, user, creator, user_initial_balance, creator_initial_balance);

        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);

        // Initially, apt payment should not be enabled
        assert!(!apt_payment::is_apt_payment_enabled(token_minter), 0);

        token_minter::add_or_update_apt_payment_guard(creator, token_minter, apt_cost, destination);
        assert!(apt_payment::amount(token_minter) == apt_cost, 0);
        assert!(apt_payment::destination(token_minter) == destination, 0);

        // Now, apt payment should be enabled
        assert!(apt_payment::is_apt_payment_enabled(token_minter), 0);

        let amount = 1;
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            amount,
            vector[vector<String>[string::utf8(b"attack")]],
            vector[vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)]],
            vector[signer::address_of(user)],
        );

        // Check balances after payment
        let total_apt_cost = apt_cost * amount;
        assert!(coin::balance<AptosCoin>(signer::address_of(user)) == user_initial_balance - total_apt_cost, 0);
        // Assert the guard `apt_payment.destination` received the APT
        assert!(coin::balance<AptosCoin>(destination) == creator_initial_balance + total_apt_cost, 0);
    }

    #[test(creator = @0x123, user = @0x456, fx = @0x1)]
    #[expected_failure(abort_code = 0x030002, location = token_minter::apt_payment)]
    fun test_insufficient_apt_balance_during_mint(creator: &signer, user: &signer, fx: &signer) {
        let apt_cost = 50;
        let destination = signer::address_of(creator);
        let user_initial_balance = 10;
        let creator_initial_balance = 0;
        setup_test_environment(fx, user, creator, user_initial_balance, creator_initial_balance);

        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        token_minter::add_or_update_apt_payment_guard(creator, token_minter, apt_cost, destination);

        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            1,
            vector[vector<String>[string::utf8(b"attack")]],
            vector[vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)]],
            vector[signer::address_of(user)],
        );
    }

    #[test(creator = @0x123, user = @0x456, fx = @0x1)]
    fun test_remove_and_execute_apt_payment(creator: &signer, user: &signer, fx: &signer) {
        let apt_cost = 50;
        let initial_balance = 100;
        let destination = signer::address_of(creator);
        let user_initial_balance = 100;
        let creator_initial_balance = 0;
        setup_test_environment(fx, user, creator, user_initial_balance, creator_initial_balance);

        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        token_minter::add_or_update_apt_payment_guard(creator, token_minter, apt_cost, destination);
        token_minter::remove_apt_payment_guard(creator, token_minter);

        assert!(!apt_payment::is_apt_payment_enabled(token_minter), 0);

        // Mint should succeed as apt payment guard is removed.
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            1,
            vector[vector<String>[string::utf8(b"attack")]],
            vector[vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)]],
            vector[signer::address_of(user)],
        );

        // Expect no APT payment made
        assert!(coin::balance<AptosCoin>(signer::address_of(user)) == initial_balance, 0);
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x10006, location = token_minter::token_minter)]
    fun test_non_creator_adding_apt_payment_guard(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        // Attempt to add APT payment guard with non-creator signer should fail.
        token_minter::add_or_update_apt_payment_guard(user, token_minter, 50, signer::address_of(creator));
    }
}
