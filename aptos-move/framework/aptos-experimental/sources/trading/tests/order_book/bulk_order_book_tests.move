#[test_only]
module aptos_experimental::bulk_order_book_tests {
    use aptos_experimental::order_book_types::{OrderMatch, new_ascending_id_generator, AscendingIdGenerator,
        bulk_order_book_type
    };
    use aptos_experimental::bulk_order_book::{BulkOrderBook, new_bulk_order_book};
    use std::vector;
    use aptos_experimental::bulk_order_book_types::{BulkOrderRequest, new_bulk_order_request};
    use aptos_experimental::price_time_index;

    // Test accounts
    const TEST_ACCOUNT_1: address = @0x1;
    const TEST_ACCOUNT_2: address = @0x2;
    const TEST_ACCOUNT_3: address = @0x3;

    // Test prices and sizes
    const BID_PRICE_1: u64 = 100;
    const BID_PRICE_2: u64 = 99;
    const ASK_PRICE_1: u64 = 101;
    const ASK_PRICE_2: u64 = 102;
    const SIZE_1: u64 = 10;
    const SIZE_2: u64 = 20;
    const SIZE_3: u64 = 15;
    const SIZE_4: u64 = 25;

    // Test metadata type for testing
    struct TestMetadata has store, copy, drop {
        test_field: u64
    }

    public(friend) fun new_test_metadata(test_field: u64): TestMetadata {
        TestMetadata { test_field }
    }

    fun setup_test(): (BulkOrderBook<TestMetadata>, price_time_index::PriceTimeIndex, AscendingIdGenerator) {
        let order_book = new_bulk_order_book<TestMetadata>();
        let price_time_idx = price_time_index::new_price_time_idx();
        let ascending_id_generator = new_ascending_id_generator();

        // Place an order first
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1,
            1, // sequence number 1 for initial setup
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        order_book.place_bulk_order(&mut price_time_idx, &mut ascending_id_generator, order_request);
        (order_book, price_time_idx, ascending_id_generator)
    }

    fun create_test_order_request(
        account: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>
    ): BulkOrderRequest<TestMetadata> {
        new_bulk_order_request(
            account,
            1, // sequence number for tests
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            new_test_metadata(1)
        )
    }

    fun create_test_order_request_with_sequence(
        account: address,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>
    ): BulkOrderRequest<TestMetadata> {
        new_bulk_order_request(
            account,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            new_test_metadata(1)
        )
    }

    fun place_taker_order_and_get_matches(
        order_book: &mut BulkOrderBook<TestMetadata>,
        price_time_index: &mut price_time_index::PriceTimeIndex,
        taker_price: u64,
        taker_size: u64,
        is_bid: bool
    ): vector<OrderMatch<TestMetadata>> {
        let match_results = vector::empty();
        // If the order is not a taker order, we return an empty match result
        let remaining_size = taker_size;
        while (remaining_size > 0) {
            // print(&order_book.is_taker_order(taker_price, is_bid));
            if (!price_time_index.is_taker_order(taker_price, is_bid)) {
                return match_results; // No matches found
            };
            let result = price_time_index.get_single_match_result(
                taker_price, remaining_size, is_bid
            );
            let match_result =
                order_book.get_single_match_for_taker(price_time_index, result, is_bid);
            let matched_size = match_result.get_matched_size();
            match_results.push_back(match_result);
            remaining_size -= matched_size;
        };
        return match_results
    }

    // Helper functions for verifying match results

    /// Verifies a single match result with expected properties
    fun verify_single_match(
        match_result: OrderMatch<TestMetadata>,
        expected_account: address,
        expected_price: u64,
        expected_orig_size: u64,
        expected_matched_size: u64,
        expected_is_bid: bool,
        expected_remaining_size: u64
    ) {
        let (matched_order, matched_size) = match_result.destroy_order_match();
        let (_order_id, account, _client_order_id, _unique_priority_idx, price, orig_size, remaining_size, is_bid, _, _metadata, order_book_type) =
            matched_order.destroy_order_match_details();

        assert!(account == expected_account);
        assert!(price == expected_price);
        assert!(orig_size == expected_orig_size);
        assert!(matched_size == expected_matched_size);
        assert!(is_bid == expected_is_bid);
        assert!(remaining_size == expected_remaining_size);
        assert!(order_book_type == bulk_order_book_type()); // Ensure it's a bulk order book match
    }

    /// Verifies a single match result with basic properties (account, price, matched_size, is_bid)
    fun verify_basic_match(
        match_result: OrderMatch<TestMetadata>,
        expected_account: address,
        expected_price: u64,
        expected_matched_size: u64,
        expected_is_bid: bool
    ) {
        let (matched_order, matched_size) = match_result.destroy_order_match();
        let (_order_id, account, _client_order_id, _unique_priority_idx, price, _orig_size, _remaining_size, is_bid, _, _metadata, order_book_type) =
            matched_order.destroy_order_match_details();

        assert!(account == expected_account);
        assert!(price == expected_price);
        assert!(matched_size == expected_matched_size);
        assert!(is_bid == expected_is_bid);
        assert!(order_book_type == bulk_order_book_type()); // Ensure it's a bulk order book match
    }

    /// Verifies total matched size across all matches
    fun verify_total_matched_size(
        matches: vector<OrderMatch<TestMetadata>>,
        expected_total_size: u64
    ) {
        let total_matched = 0u64;
        let i = 0;
        while (i < matches.length()) {
            let (_, matched_size) = matches[i].destroy_order_match();
            total_matched += matched_size;
            i += 1;
        };
        assert!(total_matched == expected_total_size);
    }

    /// Verifies that the last match has zero remaining size (fully consumed)
    fun verify_fully_consumed(matches: vector<OrderMatch<TestMetadata>>) {
        assert!(matches.length() > 0);
        let last_match = matches[matches.length() - 1];
        let (matched_order, _matched_size) = last_match.destroy_order_match();
        let (_order_id, _account, _client_order_id, _unique_priority_idx, _price, _orig_size, remaining_size, _is_bid, _, _metadata, _) =
            matched_order.destroy_order_match_details();
        assert!(remaining_size == 0);
    }

    /// Helper to extract match data for custom verification
    fun extract_match_data(
        match_result: OrderMatch<TestMetadata>
    ): (address, u64, u64, u64, u64, bool) {
        let (matched_order, matched_size) = match_result.destroy_order_match();
        let (_order_id, account, _client_order_id, _unique_priority_idx, price, orig_size, remaining_size, is_bid, _,  _metadata, _) =
            matched_order.destroy_order_match_details();
        (account, price, orig_size, matched_size, remaining_size, is_bid)
    }

    // ===== HELPER FUNCTIONS TO REDUCE DUPLICATION =====

    /// Struct to represent order data for multi-account scenarios
    struct OrderData has copy, drop {
        account: address,
        bid_price: u64,
        bid_size: u64,
        ask_price: u64,
        ask_size: u64,
    }

    /// Struct to represent expected match data
    struct ExpectedMatch has copy, drop {
        account: address,
        price: u64,
        matched_size: u64,
        is_bid: bool,
    }

    /// Creates and places a simple order with single price levels
    fun place_simple_order(
        order_book: &mut BulkOrderBook<TestMetadata>,
        price_time_index: &mut price_time_index::PriceTimeIndex,
        id_gen: &mut AscendingIdGenerator,
        account: address,
        bid_price: u64,
        bid_size: u64,
        ask_price: u64,
        ask_size: u64
    ) {
        let bid_prices = vector[bid_price];
        let bid_sizes = vector[bid_size];
        let ask_prices = vector[ask_price];
        let ask_sizes = vector[ask_size];

        let order_request = create_test_order_request_with_sequence(
            account,
            5, // sequence number 5 for subsequent orders
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );
        order_book.place_bulk_order(price_time_index, id_gen, order_request);
    }

    fun place_simple_order_with_sequence(
        order_book: &mut BulkOrderBook<TestMetadata>,
        price_time_index: &mut price_time_index::PriceTimeIndex,
        id_gen: &mut AscendingIdGenerator,
        account: address,
        sequence_number: u64,
        bid_price: u64,
        bid_size: u64,
        ask_price: u64,
        ask_size: u64
    ) {
        let bid_prices = vector[bid_price];
        let bid_sizes = vector[bid_size];
        let ask_prices = vector[ask_price];
        let ask_sizes = vector[ask_size];

        let order_request = create_test_order_request_with_sequence(
            account,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );
        order_book.place_bulk_order(price_time_index, id_gen, order_request);
    }

    /// Creates and places a multi-level order
    fun place_multi_level_order(
        order_book: &mut BulkOrderBook<TestMetadata>,
        price_time_index: &mut price_time_index::PriceTimeIndex,
        id_gen: &mut AscendingIdGenerator,
        account: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>
    ) {
        let order_request = create_test_order_request_with_sequence(
            account,
            6, // sequence number 6 for multi-level orders
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );
        order_book.place_bulk_order(price_time_index, id_gen, order_request);
    }

    /// Verifies a match with basic properties (account, price, matched_size, is_bid)
    fun verify_match_basic(
        match_result: OrderMatch<TestMetadata>,
        expected_account: address,
        expected_price: u64,
        expected_matched_size: u64,
        expected_is_bid: bool
    ) {
        let (matched_order_result, matched_size) = match_result.destroy_order_match();
        let (_, account, _, _, price, _, _, is_bid, _, _, _) = matched_order_result.destroy_order_match_details();

        assert!(account == expected_account);
        assert!(price == expected_price);
        assert!(matched_size == expected_matched_size);
        assert!(is_bid == expected_is_bid);
    }

    /// Verifies a sequence of matches with expected properties
    fun verify_match_sequence(
        matches: vector<OrderMatch<TestMetadata>>,
        expected_sequence: vector<ExpectedMatch>
    ) {
        assert!(matches.length() == expected_sequence.length());
        let i = 0;
        while (i < matches.length()) {
            let expected = expected_sequence[i];
            verify_match_basic(matches[i], expected.account, expected.price, expected.matched_size, expected.is_bid);
            i += 1;
        };
    }

    /// Creates a test scenario with multiple accounts placing orders
    fun setup_multi_account_scenario(
        order_book: &mut BulkOrderBook<TestMetadata>,
        price_time_index: &mut price_time_index::PriceTimeIndex,
        id_gen: &mut AscendingIdGenerator,
        accounts_and_orders: vector<OrderData>
    ) {
        let i = 0;
        while (i < accounts_and_orders.length()) {
            let order_data = accounts_and_orders[i];
            place_simple_order(
                order_book,
                price_time_index,
                id_gen,
                order_data.account,
                order_data.bid_price,
                order_data.bid_size,
                order_data.ask_price,
                order_data.ask_size
            );
            i += 1;
        };
    }

    /// Helper to create an OrderData struct
    fun create_order_data(
        account: address,
        bid_price: u64,
        bid_size: u64,
        ask_price: u64,
        ask_size: u64
    ): OrderData {
        OrderData {
            account,
            bid_price,
            bid_size,
            ask_price,
            ask_size,
        }
    }

    /// Helper to create an ExpectedMatch struct
    fun create_expected_match(
        account: address,
        price: u64,
        matched_size: u64,
        is_bid: bool
    ): ExpectedMatch {
        ExpectedMatch {
            account,
            price,
            matched_size,
            is_bid,
        }
    }

    #[test]
    fun test_basic_matching_for_taker_bid() {
        // Test descending order validation
        let (order_book, price_time_index, _id_gen) = setup_test();

        // First match - should match first ask level
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2, // Taker bid
            true // Taker bid
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            SIZE_1,
            false, // Should be an ask order
            SIZE_2 // Remaining size after first level consumed
        );

        // Second attempt - should be no matches since first level is consumed
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2, // Taker bid
            true // Taker bid
        );
        assert!(matches.length() == 0);

        // Third match - should match second ask level
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_2,
            SIZE_1 + SIZE_2, // Taker bid
            true // Taker bid
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_2,
            SIZE_1 + SIZE_2,
            SIZE_2,
            false, // Should be an ask order
            0 // Fully consumed
        );

        order_book.destroy_bulk_order_book();
        price_time_index.destroy_price_time_idx();
    }

    #[test]
    fun test_basic_matching_for_taker_ask() {
        // Test taker ask matching against maker bid
        let (order_book, price_time_index, _id_gen) = setup_test();

        // First match - should match first bid level
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            BID_PRICE_1,
            SIZE_1 + SIZE_2, // Taker ask
            false // Taker ask
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            BID_PRICE_1,
            SIZE_1 + SIZE_2,
            SIZE_1,
            true, // Should be a bid order
            SIZE_2 // Remaining size after first level consumed
        );

        // Second attempt - should be no matches since first level is consumed
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            BID_PRICE_1,
            SIZE_1 + SIZE_2, // Taker bid
            true // Taker bid
        );
        assert!(matches.length() == 0);

        // Third match - should match second bid level
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            BID_PRICE_2,
            SIZE_1 + SIZE_2, // Taker ask
            false // Taker ask
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            BID_PRICE_2,
            SIZE_1 + SIZE_2,
            SIZE_2,
            true, // Should be a bid order
            0 // Fully consumed
        );

        order_book.destroy_bulk_order_book();
        price_time_index.destroy_price_time_idx();
    }

    #[test]
    fun test_taker_size_smaller_than_active_bulk_order_size() {
        // Test scenario 1: Taker size is smaller than the active size of the bulk order
        let (order_book, price_time_index, _id_gen) = setup_test();

        // Taker bid with size smaller than the first ask level
        let taker_size = SIZE_1 / 2; // Half of the first ask level size
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            taker_size,
            true // Taker bid
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            taker_size,
            false, // Should be an ask order
            SIZE_1 + SIZE_2 - taker_size // Remaining size after partial consumption
        );

        // Test taker ask with size smaller than the first bid level
        let taker_ask_size = SIZE_1 / 3; // One third of the first bid level size
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            BID_PRICE_1,
            taker_ask_size,
            false // Taker ask
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            BID_PRICE_1,
            SIZE_1 + SIZE_2,
            taker_ask_size,
            true, // Should be a bid order
            SIZE_1 + SIZE_2 - taker_ask_size // Remaining size after partial consumption
        );

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_taker_size_greater_than_total_bulk_order_size() {
        // Test scenario 2: Taker size is greater than the total bulk order size
        let (order_book, price_time_index, _id_gen) = setup_test();

        let total_bulk_order_size = SIZE_1 + SIZE_2;
        let taker_size = total_bulk_order_size + 5; // Larger than total bulk order size

        // Taker bid larger than total ask size
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_2,
            taker_size,
            true // Taker bid
        );
        assert!(matches.length() == 2);

        // Check first match (first ask level)
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            total_bulk_order_size,
            SIZE_1,
            false, // Should be an ask order
            SIZE_2 // Remaining size after first level
        );

        // Check second match (second ask level)
        verify_single_match(
            matches[1],
            TEST_ACCOUNT_1,
            ASK_PRICE_2,
            total_bulk_order_size,
            SIZE_2,
            false, // Should be an ask order
            0 // Fully consumed
        );

        // Verify total matched size equals total bulk order size
        verify_total_matched_size(matches, total_bulk_order_size);

        // Verify the bulk order is fully consumed
        verify_fully_consumed(matches);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_taker_size_exactly_equal_to_bulk_order_size() {
        // Test scenario: Taker size exactly equals the total bulk order size
        let (order_book, price_time_index, _id_gen) = setup_test();
        let total_bulk_order_size = SIZE_1 + SIZE_2;

        // Taker bid exactly equal to total ask size
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_2,
            total_bulk_order_size,
            true // Taker bid
        );
        assert!(matches.length() == 2);

        // Check first match
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            total_bulk_order_size,
            SIZE_1,
            false, // Should be an ask order
            SIZE_2 // Remaining size after first level
        );

        // Check second match
        verify_single_match(
            matches[1],
            TEST_ACCOUNT_1,
            ASK_PRICE_2,
            total_bulk_order_size,
            SIZE_2,
            false, // Should be an ask order
            0 // Fully consumed
        );

        // Verify total matched size equals total bulk order size
        verify_total_matched_size(matches, total_bulk_order_size);
        verify_fully_consumed(matches);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_partial_matching_across_multiple_levels() {
        // Test partial matching that spans multiple price levels
        let (order_book, price_time_index, _id_gen) = setup_test();


        let partial_size = SIZE_1 + (SIZE_2 / 2); // First level + half of second level

        // Taker bid that partially consumes second level
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_2,
            partial_size,
            true // Taker bid
        );
        assert!(matches.length() == 2);

        // Check first match (fully consumed)
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            SIZE_1,
            false, // Should be an ask order
            SIZE_2 // Remaining size after first level
        );

        // Check second match (partially consumed)
        verify_single_match(
            matches[1],
            TEST_ACCOUNT_1,
            ASK_PRICE_2,
            SIZE_1 + SIZE_2,
            SIZE_2 / 2,
            false, // Should be an ask order
            SIZE_2 / 2 // Remaining size in second level
        );

        // Verify total matched size equals partial size
        verify_total_matched_size(matches, partial_size);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_normal_cancellation() {
        // Scenario 1: Normal cancellation - place maker order, cancel it, then try to match
        let (order_book, price_time_index, _id_gen) = setup_test();


        // Verify order is active before cancellation
        assert!(price_time_index.is_taker_order(ASK_PRICE_1, true)); // Taker bid can match ask
        assert!(price_time_index.is_taker_order(BID_PRICE_1, false)); // Taker ask can match bid

        // Cancel the order
        order_book.cancel_bulk_order(&mut price_time_index, TEST_ACCOUNT_1);

        // Verify order is no longer active after cancellation
        assert!(!price_time_index.is_taker_order(ASK_PRICE_1, true)); // Taker bid cannot match ask
        assert!(!price_time_index.is_taker_order(BID_PRICE_1, false)); // Taker ask cannot match bid

        // Try to place taker orders - should return empty
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            true // Taker bid
        );
        assert!(matches.length() == 0);

        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            BID_PRICE_1,
            SIZE_1 + SIZE_2,
            false // Taker ask
        );
        assert!(matches.length() == 0);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_cancel_after_partial_fill() {
        // Scenario 2: Cancel after partial fill
        let (order_book, price_time_index, _id_gen) = setup_test();

        // Partially match the order (consume first ask level)
        let partial_size = SIZE_1 / 2; // Half of first level
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            partial_size,
            true // Taker bid
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            partial_size,
            false, // Should be an ask order
            SIZE_1 + SIZE_2 - partial_size // Remaining size after partial consumption
        );

        // Verify order is still active (second level should still be available)
        assert!(price_time_index.is_taker_order(ASK_PRICE_2, true)); // Second ask level still active

        // Cancel the order
        order_book.cancel_bulk_order(&mut price_time_index, TEST_ACCOUNT_1);

        // Verify order is no longer active after cancellation
        assert!(!price_time_index.is_taker_order(ASK_PRICE_1, true)); // First level gone
        assert!(!price_time_index.is_taker_order(ASK_PRICE_2, true)); // Second level also gone

        // Try to place taker orders - should return empty
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            true // Taker bid
        );
        assert!(matches.length() == 0);

        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_2,
            SIZE_1 + SIZE_2,
            true // Taker bid
        );
        assert!(matches.length() == 0);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_cancel_after_full_fill() {
        // Scenario 3: Cancel after full fill
        let (order_book, price_time_index, _id_gen) = setup_test();
        // Fully match the order (consume all levels)
        let total_size = SIZE_1 + SIZE_2;
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_2, // Use second price to match both levels
            total_size,
            true // Taker bid
        );
        assert!(matches.length() == 2);

        // Verify first match
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            SIZE_1,
            false, // Should be an ask order
            SIZE_2 // Remaining size after first level
        );

        // Verify second match
        verify_single_match(
            matches[1],
            TEST_ACCOUNT_1,
            ASK_PRICE_2,
            SIZE_1 + SIZE_2,
            SIZE_2,
            false, // Should be an ask order
            0 // Fully consumed
        );

        // Verify order is no longer active after full consumption
        assert!(!price_time_index.is_taker_order(ASK_PRICE_1, true)); // First level consumed
        assert!(!price_time_index.is_taker_order(ASK_PRICE_2, true)); // Second level consumed

        // Cancel the order (should be a no-op since it's already fully consumed)
        order_book.cancel_bulk_order(&mut price_time_index, TEST_ACCOUNT_1);

        // Verify order is still not active after cancellation
        assert!(!price_time_index.is_taker_order(ASK_PRICE_1, true));
        assert!(!price_time_index.is_taker_order(ASK_PRICE_2, true));

        // Try to place taker orders - should return empty
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            true // Taker bid
        );
        assert!(matches.length() == 0);

        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_2,
            SIZE_1 + SIZE_2,
            true // Taker bid
        );
        assert!(matches.length() == 0);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book::EORDER_NOT_FOUND)]
    fun test_cancel_nonexistent_order() {
        // Test cancellation of an order that doesn't exist
        let (order_book, price_time_index, _id_gen) = setup_test();

        // Try to cancel an order that doesn't exist - should abort with EORDER_NOT_FOUND
        order_book.cancel_bulk_order(&mut price_time_index, TEST_ACCOUNT_2);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_cancel_and_recreate_order() {
        // Test canceling an order and then recreating it
        let (order_book, price_time_index, id_gen) = setup_test();

        // Verify order is active
        assert!(price_time_index.is_taker_order(ASK_PRICE_1, true));
        assert!(price_time_index.is_taker_order(BID_PRICE_1, false));

        // Cancel the order
        order_book.cancel_bulk_order(&mut price_time_index, TEST_ACCOUNT_1);

        // Verify order is no longer active
        assert!(!price_time_index.is_taker_order(ASK_PRICE_1, true));
        assert!(!price_time_index.is_taker_order(BID_PRICE_1, false));

        // Recreate the order with same account
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1,
            10, // sequence number 10 for recreation
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // Verify order is active again
        assert!(price_time_index.is_taker_order(ASK_PRICE_1, true));
        assert!(price_time_index.is_taker_order(BID_PRICE_1, false));

        // Verify it can be matched
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            ASK_PRICE_1,
            SIZE_1,
            true // Taker bid
        );
        assert!(matches.length() == 1);
        verify_single_match(
            matches[0],
            TEST_ACCOUNT_1,
            ASK_PRICE_1,
            SIZE_1 + SIZE_2,
            SIZE_1,
            false, // Should be an ask order
            SIZE_2 // Remaining size
        );

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_invalid_bid_prices_not_descending() {
        // Test placing an order with bid prices not in descending order
        let (order_book, price_time_index, id_gen) = setup_test();

        // Bid prices in ascending order (invalid - should be descending)
        let bid_prices = vector[BID_PRICE_2, BID_PRICE_1]; // 99, 100 (ascending)
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to invalid bid price ordering
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_invalid_ask_prices_not_ascending() {
        // Test placing an order with ask prices not in ascending order
        let (order_book, price_time_index, id_gen) = setup_test();


        // Ask prices in descending order (invalid - should be ascending)
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_2, ASK_PRICE_1]; // 102, 101 (descending)
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to invalid ask price ordering
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_zero_bid_size() {
        // Test placing an order with zero bid size
        let (order_book, price_time_index, id_gen) = setup_test();

        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[0, SIZE_2]; // Zero size in first bid level
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to zero bid size
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_zero_ask_size() {
        // Test placing an order with zero ask size
        let (order_book, price_time_index, id_gen) = setup_test();
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, 0]; // Zero size in second ask level

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to zero ask size
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_all_zero_sizes() {
        // Test placing an order with all zero sizes
        let (order_book, price_time_index, id_gen) = setup_test();

        let bid_prices = vector[BID_PRICE_1];
        let bid_sizes = vector[0]; // All zero bid sizes
        let ask_prices = vector[ASK_PRICE_1];
        let ask_sizes = vector[0]; // All zero ask sizes

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to all zero sizes
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_mismatched_bid_prices_and_sizes() {
        // Test placing an order with mismatched bid prices and sizes lengths
        let (order_book, price_time_index, id_gen) = setup_test();
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2]; // 2 prices
        let bid_sizes = vector[SIZE_1]; // Only 1 size
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to mismatched lengths
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_mismatched_ask_prices_and_sizes() {
        // Test placing an order with mismatched ask prices and sizes lengths
        let (order_book, price_time_index, id_gen) = setup_test();


        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1]; // Only 1 price
        let ask_sizes = vector[SIZE_1, SIZE_2]; // 2 sizes

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to mismatched lengths
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_empty_bid_vectors() {
        // Test placing an order with empty bid vectors
        let (order_book, price_time_index, id_gen) = setup_test();

        let bid_prices = vector::empty<u64>(); // Empty bid prices
        let bid_sizes = vector::empty<u64>(); // Empty bid sizes
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1,
            15, // sequence number 15 for empty bid vectors test
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_duplicate_bid_prices() {
        // Test placing an order with duplicate bid prices (not strictly descending)
        let (order_book, price_time_index, id_gen) = setup_test();

        let bid_prices = vector[BID_PRICE_1, BID_PRICE_1]; // Duplicate prices
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to duplicate bid prices
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EINVLID_MM_ORDER_REQUEST)]
    fun test_duplicate_ask_prices() {
        // Test placing an order with duplicate ask prices (not strictly ascending)
        let (order_book, price_time_index, id_gen) = setup_test();

        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_1]; // Duplicate prices
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to duplicate ask prices
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EPRICE_CROSSING)]
    fun test_price_crossing() {
        // Test placing an order where bid and ask prices cross
        // This should be prevented to avoid self-matching within a single order
        let (order_book, price_time_index, id_gen) = setup_test();

        // Bid price 100, Ask price 99 - this crosses (bid > ask)
        let bid_prices = vector[100]; // Highest bid price
        let bid_sizes = vector[SIZE_1];
        let ask_prices = vector[99]; // Lowest ask price
        let ask_sizes = vector[SIZE_1];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to price crossing
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EPRICE_CROSSING)]
    fun test_price_crossing_equal_prices() {
        // Test placing an order where bid and ask prices are equal (also crossing)
        let (order_book, price_time_index, id_gen) = setup_test();
        // Bid price 100, Ask price 100 - this also crosses (bid == ask)
        let bid_prices = vector[100]; // Highest bid price
        let bid_sizes = vector[SIZE_1];
        let ask_prices = vector[100]; // Lowest ask price (same as bid)
        let ask_sizes = vector[SIZE_1];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );

        // This should abort due to price crossing (equal prices)
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        // This line should never be reached
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book_types::EPRICE_CROSSING)]
    fun test_price_crossing_multiple_levels() {
        // Test placing an order with multiple price levels where the highest bid crosses the lowest ask
        let (order_book, price_time_index, id_gen) = setup_test();

        // Bid prices: 100, 99 (descending)
        // Ask prices: 98, 99 (ascending)
        // The highest bid (100) crosses the lowest ask (98)
        let bid_prices = vector[100, 99];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[98, 99];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let order_request = create_test_order_request(
            TEST_ACCOUNT_1,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes
        );
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_spread_crossing_prevented_by_active_order_book() {
        // Test that the active_order_book prevents spread crossing when placing orders
        // This verifies that the existing validation in active_order_book.place_maker_order works
        let (order_book, price_time_index, id_gen) = setup_test();

        // Place first order: bid at 100, ask at 101
        let bid_prices_1 = vector[100];
        let bid_sizes_1 = vector[SIZE_1];
        let ask_prices_1 = vector[105];
        let ask_sizes_1 = vector[SIZE_1];

        let order_request_1 = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1,
            20, // sequence number 20 for first order
            bid_prices_1,
            bid_sizes_1,
            ask_prices_1,
            ask_sizes_1
        );
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request_1);

        let bid_prices_2 = vector[106, 105, 104]; // 106 crosses 105
        let bid_sizes_2 = vector[SIZE_1, SIZE_2, SIZE_3];
        let ask_prices_2 = vector[107, 108]; //
        let ask_sizes_2 = vector[SIZE_1, SIZE_2];

        let order_request_2 = create_test_order_request_with_sequence(
            TEST_ACCOUNT_2,
            25, // sequence number 25 for second order
            bid_prices_2,
            bid_sizes_2,
            ask_prices_2,
            ask_sizes_2
        );
        order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_request_2);

        let bid_prices = order_book.get_prices(TEST_ACCOUNT_2, true);
        let ask_prices = order_book.get_prices(TEST_ACCOUNT_2, false);
        let bid_sizes = order_book.get_sizes(TEST_ACCOUNT_2, true);
        let ask_sizes = order_book.get_sizes(TEST_ACCOUNT_2, false);

        // 102 bid should be rejected,
        assert!(bid_prices.length() == 1);
        assert!(bid_prices[0] == 104);
        assert!(bid_sizes.length() == 1);
        assert!(bid_sizes[0] == SIZE_3);

        // 99 ask should be rejected
        assert!(ask_prices.length() == 2);
        assert!(ask_prices[0] == 107);
        assert!(ask_prices[1] == 108);
        assert!(ask_sizes.length() == 2);
        assert!(ask_sizes[0] == SIZE_1);
        assert!(ask_sizes[1] == SIZE_2);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    // ===== MULTI-ACCOUNT TESTS =====

    #[test]
    fun test_two_accounts_same_price_level() {
        // Test two accounts placing orders at the same price level
        // Should match in order of placement (time priority)
        let (order_book, price_time_index, id_gen) = setup_test();

        // Setup scenario: two accounts with same ask price
        let accounts_and_orders = vector[
            create_order_data(TEST_ACCOUNT_1, 100, SIZE_1, 101, SIZE_1), // Account 1 places first
            create_order_data(TEST_ACCOUNT_2, 99, SIZE_2, 101, SIZE_2)   // Account 2 places second (same ask price)
        ];
        setup_multi_account_scenario(&mut order_book, &mut price_time_index, &mut id_gen, accounts_and_orders);

        // Taker bid should match account 2 first (better priority index since placed later)
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            101, // Ask price
            SIZE_2, // Match exactly account 2's size
            true // Taker bid
        );

        assert!(matches.length() == 1);
        verify_match_basic(matches[0], TEST_ACCOUNT_2, 101, SIZE_2, false);

        // Account 1 should still be active
        assert!(price_time_index.is_taker_order(101, true));

        // Next match should be account 1
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            101,
            SIZE_1,
            true // Taker bid
        );

        assert!(matches.length() == 1);
        verify_match_basic(matches[0], TEST_ACCOUNT_1, 101, SIZE_1, false);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_three_accounts_different_price_levels() {
        // Test three accounts with different price levels
        // Should match in price priority order
        let (order_book, price_time_index, id_gen) = setup_test();

        // Setup scenario: three accounts with different ask prices
        let accounts_and_orders = vector[
            create_order_data(TEST_ACCOUNT_1, 99, SIZE_1, 101, SIZE_1), // Best ask price (101)
            create_order_data(TEST_ACCOUNT_2, 98, SIZE_2, 102, SIZE_2), // Second best ask price (102)
            create_order_data(TEST_ACCOUNT_3, 97, SIZE_3, 103, SIZE_3)  // Third best ask price (103)
        ];
        setup_multi_account_scenario(&mut order_book, &mut price_time_index, &mut id_gen, accounts_and_orders);

        // Large taker bid should match all three accounts in price priority order
        let total_size = SIZE_1 + SIZE_2 + SIZE_3;
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            103, // High enough price to match all levels
            total_size,
            true // Taker bid
        );

        assert!(matches.length() == 3);

        // Verify matches in price priority order
        let expected_sequence = vector[
            create_expected_match(TEST_ACCOUNT_1, 101, SIZE_1, false), // First: best price
            create_expected_match(TEST_ACCOUNT_2, 102, SIZE_2, false), // Second: second best price
            create_expected_match(TEST_ACCOUNT_3, 103, SIZE_3, false)  // Third: third best price
        ];
        verify_match_sequence(matches, expected_sequence);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_multiple_accounts_partial_fills() {
        // Test partial fills across multiple accounts
        let (order_book, price_time_index, id_gen) = setup_test();

        // Setup scenario: two accounts with different ask prices
        let accounts_and_orders = vector[
            create_order_data(TEST_ACCOUNT_1, 99, SIZE_1, 101, SIZE_1), // 10 units at price 101
            create_order_data(TEST_ACCOUNT_2, 98, SIZE_2, 102, SIZE_2)  // 20 units at price 102
        ];
        setup_multi_account_scenario(&mut order_book, &mut price_time_index, &mut id_gen, accounts_and_orders);

        // Taker bid for 25 units (should fill account 1 completely and partially fill account 2)
        let taker_size = SIZE_1 + SIZE_3; // 10 + 15 = 25
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            102,
            taker_size,
            true // Taker bid
        );

        assert!(matches.length() == 2);

        // Verify matches: account 1 fully filled, account 2 partially filled
        let expected_sequence = vector[
            create_expected_match(TEST_ACCOUNT_1, 101, SIZE_1, false), // First: account 1 fully filled (10 units)
            create_expected_match(TEST_ACCOUNT_2, 102, SIZE_3, false)  // Second: account 2 partially filled (15 units)
        ];
        verify_match_sequence(matches, expected_sequence);

        // Account 2 should still be active with remaining size
        assert!(price_time_index.is_taker_order(102, true));

        // Verify remaining size in account 2
        let remaining_matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            102,
            SIZE_2 - SIZE_3, // Remaining size: 20 - 15 = 5
            true
        );
        assert!(remaining_matches.length() == 1);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_bid_and_ask_matching_multiple_accounts() {
        // Test both bid and ask matching with multiple accounts
        let (order_book, price_time_index, id_gen) = setup_test();

        // Setup scenario: two accounts with different bid/ask prices
        let accounts_and_orders = vector[
            create_order_data(TEST_ACCOUNT_1, 100, SIZE_1, 101, SIZE_1), // Bid at 100, Ask at 101
            create_order_data(TEST_ACCOUNT_2, 99, SIZE_2, 102, SIZE_2)   // Bid at 99, Ask at 102
        ];
        setup_multi_account_scenario(&mut order_book, &mut price_time_index, &mut id_gen, accounts_and_orders);

        // Test taker bid matching against asks
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            101,
            SIZE_1,
            true // Taker bid
        );

        assert!(matches.length() == 1);
        verify_match_basic(matches[0], TEST_ACCOUNT_1, 101, SIZE_1, false);

        // Test taker ask matching against bids
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            100,
            SIZE_1,
            false // Taker ask
        );

        assert!(matches.length() == 1);
        verify_match_basic(matches[0], TEST_ACCOUNT_1, 100, SIZE_1, true);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_cancellation_with_multiple_accounts() {
        // Test cancellation behavior with multiple accounts
        let (order_book, price_time_index, id_gen) = setup_test();


        // Setup scenario: two accounts with different prices
        let accounts_and_orders = vector[
            create_order_data(TEST_ACCOUNT_1, 100, SIZE_1, 101, SIZE_1), // Account 1: bid at 100, ask at 101
            create_order_data(TEST_ACCOUNT_2, 99, SIZE_2, 102, SIZE_2)   // Account 2: bid at 99, ask at 102
        ];
        setup_multi_account_scenario(&mut order_book, &mut price_time_index, &mut id_gen, accounts_and_orders);

        // Verify both orders are active
        assert!(price_time_index.is_taker_order(101, true)); // Account 1's ask
        assert!(price_time_index.is_taker_order(102, true)); // Account 2's ask
        assert!(price_time_index.is_taker_order(100, false)); // Account 1's bid
        assert!(price_time_index.is_taker_order(99, false)); // Account 2's bid

        // Cancel account 1's order
        order_book.cancel_bulk_order(&mut price_time_index, TEST_ACCOUNT_1);

        // Account 1's orders should no longer be active
        assert!(!price_time_index.is_taker_order(101, true)); // Account 1's ask
        assert!(!price_time_index.is_taker_order(100, false)); // Account 1's bid

        // Account 2's orders should still be active
        assert!(price_time_index.is_taker_order(102, true)); // Account 2's ask
        assert!(price_time_index.is_taker_order(99, false)); // Account 2's bid

        // Verify we can still match against account 2
        let matches = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            102,
            SIZE_2,
            true // Taker bid
        );

        assert!(matches.length() == 1);
        verify_match_basic(matches[0], TEST_ACCOUNT_2, 102, SIZE_2, false);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_order_replacement_multiple_accounts() {
        // Test order replacement behavior with multiple accounts
        let (order_book, price_time_index, id_gen) = setup_test();

        // Setup initial scenario: two accounts
        let accounts_and_orders = vector[
            create_order_data(TEST_ACCOUNT_1, 100, SIZE_1, 101, SIZE_1), // Account 1: bid at 100, ask at 101
            create_order_data(TEST_ACCOUNT_2, 99, SIZE_2, 102, SIZE_2)   // Account 2: bid at 99, ask at 102
        ];
        setup_multi_account_scenario(&mut order_book, &mut price_time_index, &mut id_gen, accounts_and_orders);

        // Account 1 replaces their order with different prices
        place_simple_order_with_sequence(&mut order_book, &mut price_time_index, &mut id_gen, TEST_ACCOUNT_1, 30, 98, SIZE_3, 103, SIZE_3);

        // Old prices should no longer be active
        assert!(!price_time_index.is_taker_order(101, true)); // Old ask price
        assert!(!price_time_index.is_taker_order(100, false)); // Old bid price

        // New prices should be active
        assert!(price_time_index.is_taker_order(103, true)); // New ask price
        assert!(price_time_index.is_taker_order(98, false)); // New bid price

        // Account 2's orders should still be active
        assert!(price_time_index.is_taker_order(102, true)); // Account 2's ask
        assert!(price_time_index.is_taker_order(99, false)); // Account 2's bid

        // Verify we can match against both orders
        let matches_1 = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            103,
            SIZE_3,
            true // Taker bid
        );
        assert!(matches_1.length() == 1);

        let matches_2 = place_taker_order_and_get_matches(
            &mut order_book,
            &mut price_time_index,
            102,
            SIZE_2,
            true // Taker bid
        );
        assert!(matches_2.length() == 1);

        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    fun test_sequence_number_validation() {
        let (order_book, price_time_index, id_gen) = setup_test();
        // Test that we can place an order with higher sequence number (replacing the one from setup_test)
        let order_req1 = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1, 10, vector[100], vector[10], vector[200], vector[10]
        );
        let _order_id1 = order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_req1);
        // Test that we can place an order with even higher sequence number
        let order_req2 = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1, 15, vector[100], vector[10], vector[200], vector[10]
        );
        let _order_id2 = order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_req2);
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }

    #[test]
    #[expected_failure(abort_code = aptos_experimental::bulk_order_book::E_INVALID_SEQUENCE_NUMBER)]
    fun test_sequence_number_validation_failure() {
        let (order_book, price_time_index, id_gen) = setup_test();
        // Test that we can place an order with higher sequence number (replacing the one from setup_test)
        let order_req1 = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1, 10, vector[100], vector[10], vector[200], vector[10]
        );
        let _order_id1 = order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_req1);
        // Test that we can place an order with even higher sequence number
        let order_req2 = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1, 15, vector[100], vector[10], vector[200], vector[10]
        );
        let _order_id2 = order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_req2);
        // Test that we cannot place an order with lower sequence number (should abort)
        let order_req3 = create_test_order_request_with_sequence(
            TEST_ACCOUNT_1, 12, vector[100], vector[10], vector[200], vector[10]
        );
        let _ = order_book.place_bulk_order(&mut price_time_index, &mut id_gen, order_req3);
        price_time_index.destroy_price_time_idx();
        order_book.destroy_bulk_order_book();
    }
}
