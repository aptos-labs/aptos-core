module aptos_trading::bulk_order_types_tests {
    #[test_only]
    use aptos_trading::bulk_order_types::new_bulk_order_request;
    #[test_only]
    use aptos_trading::order_book_types::new_test_metadata;

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
    const TOTAL_SIZE_PER_SIDE: u64 = SIZE_1 + SIZE_2;

    #[test]
    #[expected_failure(abort_code = aptos_trading::bulk_order_types::E_BID_ORDER_INVALID)]
    fun test_invalid_bid_prices_not_descending() {
        // Test placing an order with bid prices not in descending order - should return rejection
        // Bid prices in ascending order (invalid - should be descending)
        let bid_prices = vector[BID_PRICE_2, BID_PRICE_1]; // 99, 100 (ascending)
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let _response =
            new_bulk_order_request(
                TEST_ACCOUNT_1,
                1,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_metadata()
            );
    }

    #[test]
    #[expected_failure(abort_code = aptos_trading::bulk_order_types::E_ASK_ORDER_INVALID)]
    fun test_invalid_ask_prices_not_ascending() {
        // Test placing an order with ask prices not in ascending order - should return rejection
        // Ask prices in descending order (invalid - should be ascending)
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_2, ASK_PRICE_1]; // 102, 101 (descending)
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let _response =
            new_bulk_order_request(
                TEST_ACCOUNT_1,
                1,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_metadata()
            );
    }

    #[test]
    #[expected_failure(abort_code = aptos_trading::bulk_order_types::E_BID_SIZE_ZERO)]
    fun test_zero_bid_size() {
        // Test placing an order with zero bid size - should return rejection
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[0, SIZE_2]; // Zero size in first bid level
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let _response =
            new_bulk_order_request(
                TEST_ACCOUNT_1,
                1,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_metadata()
            );
    }

    #[test]
    #[expected_failure(abort_code = aptos_trading::bulk_order_types::E_ASK_SIZE_ZERO)]
    fun test_zero_ask_size() {
        // Test placing an order with zero ask size - should return rejection
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, 0]; // Zero size in second ask level

        let _response =
            new_bulk_order_request(
                TEST_ACCOUNT_1,
                1,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_metadata()
            );
    }

    #[test]
    #[expected_failure(abort_code = aptos_trading::bulk_order_types::E_BID_SIZE_ZERO)]
    fun test_all_zero_sizes() {
        // Test placing an order with all zero sizes - should return rejection
        let bid_prices = vector[BID_PRICE_1];
        let bid_sizes = vector[0]; // All zero bid sizes
        let ask_prices = vector[ASK_PRICE_1];
        let ask_sizes = vector[0]; // All zero ask sizes

        let _response =
            new_bulk_order_request(
                TEST_ACCOUNT_1,
                1,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_metadata()
            );
    }

    #[test]
    #[expected_failure(
        abort_code = aptos_trading::bulk_order_types::E_BID_LENGTH_MISMATCH
    )]
    fun test_mismatched_bid_prices_and_sizes() {
        // Test placing an order with mismatched bid prices and sizes lengths - should return rejection
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2]; // 2 prices
        let bid_sizes = vector[SIZE_1]; // Only 1 size
        let ask_prices = vector[ASK_PRICE_1, ASK_PRICE_2];
        let ask_sizes = vector[SIZE_1, SIZE_2];

        let _response =
            new_bulk_order_request(
                TEST_ACCOUNT_1,
                1,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_metadata()
            );
    }

    #[test]
    #[expected_failure(
        abort_code = aptos_trading::bulk_order_types::E_ASK_LENGTH_MISMATCH
    )]
    fun test_mismatched_ask_prices_and_sizes() {
        // Test placing an order with mismatched ask prices and sizes lengths - should return rejection
        let bid_prices = vector[BID_PRICE_1, BID_PRICE_2];
        let bid_sizes = vector[SIZE_1, SIZE_2];
        let ask_prices = vector[ASK_PRICE_1]; // Only 1 price
        let ask_sizes = vector[SIZE_1, SIZE_2]; // 2 sizes

        let _response =
            new_bulk_order_request(
                TEST_ACCOUNT_1,
                1,
                bid_prices,
                bid_sizes,
                ask_prices,
                ask_sizes,
                new_test_metadata()
            );
    }
}
