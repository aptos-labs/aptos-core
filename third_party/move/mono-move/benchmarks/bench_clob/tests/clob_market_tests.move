#[test_only]
module bench::clob_market_tests {
    use std::signer;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::Object;
    use bench::clob_market;
    use bench::clob_mock_fa;

    const BID: bool = true;
    const ASK: bool = false;

    const LOT: u64 = 100;
    const TICK: u64 = 10;

    /// Enough of both assets that no test runs out of collateral.
    const FUNDING: u64 = 1000000000;
    /// Every book in these tests is far smaller than this.
    const ALL: u64 = 1000;

    const LCG_MOD: u64 = 1000003;
    const CHECKSUM_MOD: u64 = 1000000007;

    fun make_market(admin: &signer): (u64, Object<Metadata>, Object<Metadata>) {
        let base = clob_mock_fa::create_asset(admin, b"BASE", 8);
        let quote = clob_mock_fa::create_asset(admin, b"QUOTE", 6);
        let market_id =
            clob_market::register_market_id(admin, base, quote, LOT, TICK);
        (market_id, base, quote)
    }

    fun fund(
        admin: &signer,
        user: &signer,
        market_id: u64,
        base: Object<Metadata>,
        quote: Object<Metadata>,
    ) {
        let addr = signer::address_of(user);
        clob_market::register_market_account(user, market_id);
        clob_mock_fa::mint(admin, base, addr, FUNDING);
        clob_mock_fa::mint(admin, quote, addr, FUNDING);
        clob_market::deposit(user, market_id, base, FUNDING);
        clob_market::deposit(user, market_id, quote, FUNDING);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_end_to_end(admin: &signer, alice: &signer, bob: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, admin, market_id, base, quote);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::seed_book(admin, market_id, 8, 8, 1000, 5, 42);
        assert!(clob_market::n_orders(market_id, BID, ALL) == 8, 0);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 8, 0);
        // The seeded sides must straddle the base price.
        assert!(clob_market::best_price(market_id, BID) <= 995, 0);
        assert!(clob_market::best_price(market_id, ASK) >= 1005, 0);

        let asks_before = clob_market::resting_size(market_id, ASK, ALL);
        let (base_before, _, _, _) =
            clob_market::account_state(signer::address_of(alice), market_id);
        clob_market::place_market_order(alice, market_id, BID, 3);
        assert!(
            clob_market::resting_size(market_id, ASK, ALL) == asks_before - 3,
            0,
        );
        let (base_after, _, _, _) =
            clob_market::account_state(signer::address_of(alice), market_id);
        assert!(base_after == base_before + 3 * LOT, 0);

        // Rest an order well away from the book, then take it back off.
        let n_asks = clob_market::n_orders(market_id, ASK, ALL);
        let id = clob_market::place_limit_order_id(bob, market_id, ASK, 2000, 4);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == n_asks + 1, 0);
        clob_market::cancel_order(bob, market_id, id);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == n_asks, 0);

        assert!(clob_market::index_orders(market_id, ASK, ALL) != 0, 0);
        assert!(clob_market::index_orders(market_id, BID, ALL) != 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_partial_fill_leaves_reduced_order(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 10);
        let alice_addr = signer::address_of(alice);
        let (_, base_locked, _, _) =
            clob_market::account_state(alice_addr, market_id);
        assert!(base_locked == 10 * LOT, 0);

        clob_market::place_market_order(bob, market_id, BID, 4);
        // One order still rests, at the same price, six lots lighter.
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 1, 0);
        assert!(clob_market::best_price(market_id, ASK) == 100, 0);
        assert!(clob_market::resting_size(market_id, ASK, ALL) == 6, 0);

        let (base_avail, base_locked, quote_avail, _) =
            clob_market::account_state(alice_addr, market_id);
        assert!(base_locked == 6 * LOT, 0);
        assert!(base_avail == FUNDING - 10 * LOT, 0);
        assert!(quote_avail == FUNDING + 4 * 100 * TICK, 0);

        // The assets themselves moved, not just the ledger.
        let (alice_base, alice_quote) =
            clob_market::store_balances(alice_addr, market_id);
        assert!(alice_base == FUNDING - 4 * LOT, 0);
        assert!(alice_quote == FUNDING + 4 * 100 * TICK, 0);
        let (bob_base, bob_quote) =
            clob_market::store_balances(signer::address_of(bob), market_id);
        assert!(bob_base == FUNDING + 4 * LOT, 0);
        assert!(bob_quote == FUNDING - 4 * 100 * TICK, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_crossing_limit_order_rests_remainder(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 5);
        let id = clob_market::place_limit_order_id(bob, market_id, BID, 100, 8);
        assert!(id != 0, 0);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 0, 0);
        assert!(clob_market::n_orders(market_id, BID, ALL) == 1, 0);
        assert!(clob_market::resting_size(market_id, BID, ALL) == 3, 0);
        assert!(clob_market::best_price(market_id, BID) == 100, 0);
        let (_, _, _, quote_locked) =
            clob_market::account_state(signer::address_of(bob), market_id);
        assert!(quote_locked == 3 * 100 * TICK, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_fully_filled_limit_order_rests_nothing(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 9);
        let id = clob_market::place_limit_order_id(bob, market_id, BID, 100, 4);
        assert!(id == 0, 0);
        assert!(clob_market::n_orders(market_id, BID, ALL) == 0, 0);
        assert!(clob_market::resting_size(market_id, ASK, ALL) == 5, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_limit_order_does_not_cross_past_its_price(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 5);
        clob_market::place_limit_order(alice, market_id, ASK, 110, 5);
        // Bidding 100 reaches the first ask only.
        clob_market::place_limit_order(bob, market_id, BID, 100, 9);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 1, 0);
        assert!(clob_market::best_price(market_id, ASK) == 110, 0);
        assert!(clob_market::resting_size(market_id, BID, ALL) == 4, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_cancel_leaves_rest_of_level_intact(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 1);
        let middle =
            clob_market::place_limit_order_id(alice, market_id, ASK, 100, 2);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 3);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 3, 0);

        clob_market::cancel_order(alice, market_id, middle);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 2, 0);
        assert!(clob_market::resting_size(market_id, ASK, ALL) == 4, 0);
        let (_, base_locked, _, _) =
            clob_market::account_state(signer::address_of(alice), market_id);
        assert!(base_locked == 4 * LOT, 0);

        // Taking one lot must consume the level's head, which is the size-one
        // order placed first, leaving only the size-three order.
        clob_market::place_market_order(bob, market_id, BID, 1);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 1, 0);
        assert!(clob_market::resting_size(market_id, ASK, ALL) == 3, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    #[expected_failure(abort_code = 9, location = bench::clob_market)]
    fun test_cancel_wrong_owner(admin: &signer, alice: &signer, bob: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        let id =
            clob_market::place_limit_order_id(alice, market_id, ASK, 100, 1);
        clob_market::cancel_order(bob, market_id, id);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_price_priority(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 105, 1);
        clob_market::place_limit_order(alice, market_id, ASK, 101, 1);
        clob_market::place_limit_order(alice, market_id, ASK, 103, 1);
        assert!(clob_market::best_price(market_id, ASK) == 101, 0);
        clob_market::place_limit_order(alice, market_id, BID, 95, 1);
        clob_market::place_limit_order(alice, market_id, BID, 99, 1);
        clob_market::place_limit_order(alice, market_id, BID, 97, 1);
        assert!(clob_market::best_price(market_id, BID) == 99, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_market_order_walks_price_levels(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 101, 1);
        clob_market::place_limit_order(alice, market_id, ASK, 102, 1);
        clob_market::place_limit_order(alice, market_id, ASK, 103, 1);
        clob_market::place_limit_order(alice, market_id, ASK, 104, 1);
        let bob_addr = signer::address_of(bob);
        let (_, _, quote_before, _) =
            clob_market::account_state(bob_addr, market_id);
        clob_market::place_market_order(bob, market_id, BID, 3);
        assert!(clob_market::best_price(market_id, ASK) == 104, 0);
        let (_, _, quote_after, _) =
            clob_market::account_state(bob_addr, market_id);
        // Each level charges its own price, not the best one.
        assert!(quote_before - quote_after == (101 + 102 + 103) * TICK, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_market_order_stops_when_book_empties(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 5);
        // Asking for more than the book holds fills what it can.
        clob_market::place_market_order(bob, market_id, BID, 100);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 0, 0);
        assert!(clob_market::best_price(market_id, ASK) == 0, 0);
        let (base_avail, _, _, _) =
            clob_market::account_state(signer::address_of(bob), market_id);
        assert!(base_avail == FUNDING + 5 * LOT, 0);
        // An empty book is not an error.
        clob_market::place_market_order(bob, market_id, BID, 7);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_change_order_size_shrink(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        let id =
            clob_market::place_limit_order_id(alice, market_id, ASK, 100, 9);
        let same = clob_market::change_order_size_id(alice, market_id, id, 4);
        // Shrinking keeps the order in place, so the id survives.
        assert!(same == id, 0);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 1, 0);
        assert!(clob_market::resting_size(market_id, ASK, ALL) == 4, 0);
        let (base_avail, base_locked, _, _) =
            clob_market::account_state(signer::address_of(alice), market_id);
        assert!(base_locked == 4 * LOT, 0);
        assert!(base_avail == FUNDING - 4 * LOT, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_change_order_size_grow(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        let id =
            clob_market::place_limit_order_id(alice, market_id, BID, 100, 2);
        let new_id = clob_market::change_order_size_id(alice, market_id, id, 7);
        assert!(new_id != id, 0);
        assert!(clob_market::n_orders(market_id, BID, ALL) == 1, 0);
        assert!(clob_market::resting_size(market_id, BID, ALL) == 7, 0);
        let (_, _, _, quote_locked) =
            clob_market::account_state(signer::address_of(alice), market_id);
        assert!(quote_locked == 7 * 100 * TICK, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    #[expected_failure(abort_code = 10, location = bench::clob_market)]
    fun test_stale_order_id_rejected(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        let id =
            clob_market::place_limit_order_id(alice, market_id, ASK, 100, 1);
        clob_market::cancel_order(alice, market_id, id);
        // The freed nodes come back on the next insert, so the old id now
        // addresses a different order.
        clob_market::place_limit_order(alice, market_id, ASK, 100, 1);
        clob_market::cancel_order(alice, market_id, id);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_index_orders_checksum_and_limit(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 103, 3);
        clob_market::place_limit_order(alice, market_id, ASK, 101, 1);
        clob_market::place_limit_order(alice, market_id, ASK, 102, 2);
        // The walk is in price order, so the fold is over 101, 102, 103.
        let one = (101 + 1) % CHECKSUM_MOD;
        let two = (one * LCG_MOD + 102 + 2) % CHECKSUM_MOD;
        let three = (two * LCG_MOD + 103 + 3) % CHECKSUM_MOD;
        assert!(clob_market::index_orders(market_id, ASK, 1) == one, 0);
        assert!(clob_market::index_orders(market_id, ASK, 2) == two, 0);
        assert!(clob_market::index_orders(market_id, ASK, 3) == three, 0);
        // Past the end of the book the walk stops on its own.
        assert!(clob_market::index_orders(market_id, ASK, 99) == three, 0);
        assert!(clob_market::index_orders(market_id, ASK, 0) == 0, 0);
        assert!(clob_market::index_orders(market_id, BID, 99) == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_index_orders_descending_side(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, BID, 97, 3);
        clob_market::place_limit_order(alice, market_id, BID, 99, 1);
        clob_market::place_limit_order(alice, market_id, BID, 98, 2);
        // Bids walk from the highest price down.
        let one = (99 + 1) % CHECKSUM_MOD;
        let two = (one * LCG_MOD + 98 + 2) % CHECKSUM_MOD;
        let three = (two * LCG_MOD + 97 + 3) % CHECKSUM_MOD;
        assert!(clob_market::index_orders(market_id, BID, 99) == three, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_run(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 101, 1);
        let expected = clob_market::index_orders(market_id, ASK, ALL);
        clob_market::run(admin, market_id, ASK, ALL, expected);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    #[expected_failure(abort_code = 13, location = bench::clob_market)]
    fun test_run_bad_expected(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 101, 1);
        let expected = clob_market::index_orders(market_id, ASK, ALL);
        clob_market::run(admin, market_id, ASK, ALL, expected + 1);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_assets_are_conserved(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 6);
        clob_market::place_limit_order(bob, market_id, BID, 100, 4);
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let (alice_base, alice_quote) =
            clob_market::store_balances(alice_addr, market_id);
        let (bob_base, bob_quote) =
            clob_market::store_balances(bob_addr, market_id);
        assert!(alice_base + bob_base == 2 * FUNDING, 0);
        assert!(alice_quote + bob_quote == 2 * FUNDING, 0);
        // Nothing is left outside the market accounts either.
        assert!(clob_mock_fa::primary_balance(alice_addr, base) == 0, 0);
        assert!(clob_mock_fa::primary_balance(bob_addr, quote) == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    #[expected_failure(abort_code = 8, location = bench::clob_market)]
    fun test_insufficient_collateral(admin: &signer, alice: &signer) {
        let (market_id, base, _quote) = make_market(admin);
        let alice_addr = signer::address_of(alice);
        clob_market::register_market_account(alice, market_id);
        clob_mock_fa::mint(admin, base, alice_addr, 5 * LOT);
        clob_market::deposit(alice, market_id, base, 5 * LOT);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 6);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    #[expected_failure(abort_code = 6, location = bench::clob_market)]
    fun test_zero_price_rejected(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, BID, 0, 1);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    #[expected_failure(abort_code = 7, location = bench::clob_market)]
    fun test_zero_size_rejected(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::place_limit_order(alice, market_id, ASK, 100, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    #[expected_failure(abort_code = 4, location = bench::clob_market)]
    fun test_double_registration_rejected(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        clob_market::register_market_account(alice, market_id);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    #[expected_failure(abort_code = 5, location = bench::clob_market)]
    fun test_deposit_unknown_asset(admin: &signer, alice: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        let other = clob_mock_fa::create_asset(admin, b"OTHER", 8);
        clob_mock_fa::mint(admin, other, signer::address_of(alice), 10);
        clob_market::deposit(alice, market_id, other, 10);
    }

    #[test(admin = @bench)]
    fun test_two_markets_are_independent(admin: &signer) {
        let base = clob_mock_fa::create_asset(admin, b"BASE", 8);
        let quote = clob_mock_fa::create_asset(admin, b"QUOTE", 6);
        let first =
            clob_market::register_market_id(admin, base, quote, LOT, TICK);
        let second =
            clob_market::register_market_id(admin, base, quote, LOT, TICK);
        assert!(first == 1 && second == 2, 0);
        let admin_addr = signer::address_of(admin);
        clob_market::register_market_account(admin, first);
        clob_market::register_market_account(admin, second);
        clob_mock_fa::mint(admin, base, admin_addr, FUNDING);
        clob_market::deposit(admin, first, base, FUNDING / 2);
        clob_market::deposit(admin, second, base, FUNDING / 2);
        clob_market::place_limit_order(admin, first, ASK, 100, 1);
        assert!(clob_market::n_orders(first, ASK, ALL) == 1, 0);
        assert!(clob_market::n_orders(second, ASK, ALL) == 0, 0);
    }

    #[test(admin = @bench)]
    fun test_seed_book_is_jittered(admin: &signer) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, admin, market_id, base, quote);
        clob_market::seed_book(admin, market_id, 32, 32, 5000, 10, 12345);
        assert!(clob_market::n_orders(market_id, BID, ALL) == 32, 0);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 32, 0);
        assert!(clob_market::best_price(market_id, BID) < 5000, 0);
        assert!(clob_market::best_price(market_id, ASK) > 5000, 0);
        assert!(clob_market::index_orders(market_id, BID, ALL) != 0, 0);
        // Distinct prices, so the tree is deep rather than one long list.
        assert!(clob_market::book_height(market_id, BID) >= 4, 0);
        assert!(clob_market::book_height(market_id, ASK) >= 4, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_deep_book_matches_many_levels(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        let (market_id, base, quote) = make_market(admin);
        fund(admin, alice, market_id, base, quote);
        fund(admin, bob, market_id, base, quote);
        // One lot per level, so a market order of n clears exactly n levels.
        let i = 0;
        while (i < 64) {
            clob_market::place_limit_order(alice, market_id, ASK, 1000 + i, 1);
            i = i + 1;
        };
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 64, 0);
        clob_market::place_market_order(bob, market_id, BID, 16);
        assert!(clob_market::n_orders(market_id, ASK, ALL) == 48, 0);
        assert!(clob_market::best_price(market_id, ASK) == 1016, 0);
        assert!(clob_market::resting_size(market_id, ASK, ALL) == 48, 0);
    }
}
