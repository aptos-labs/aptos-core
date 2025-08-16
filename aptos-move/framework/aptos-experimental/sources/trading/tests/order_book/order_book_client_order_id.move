#[test_only]
module aptos_experimental::order_book_client_order_id {
    use std::option;
    use std::signer;
    use aptos_experimental::order_book_types::new_order_id_type;
    use aptos_experimental::order_book_types::good_till_cancelled;
    use aptos_experimental::order_book::{new_order_request, destroy_order_book, set_up_test_with_id};

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_success(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);
        let client_order_id = 12345;
        let order_id = new_order_id_type(1);

        // Create an order request with client order ID
        let order_req =
            new_order_request(
                user1_addr,
                order_id,
                option::some(client_order_id),
                1000, // price
                100, // orig_size
                100, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );

        // Place the maker order
        order_book.place_maker_order(order_req);

        // Test 1: Successfully cancel order with correct client order ID and user
        let cancel_result =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
        assert!(cancel_result.is_some());

        // Test 2: Try to cancel the same order again - should return false
        let cancel_result_again =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
        assert!(cancel_result_again.is_none());
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_nonexistent(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);
        let nonexistent_client_order_id = 99999;

        // Test: Try to cancel a non-existent client order ID - should return false
        let cancel_result =
            order_book.try_cancel_order_with_client_order_id(user1_addr, nonexistent_client_order_id);
        assert!(cancel_result.is_none());
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456, user2 = @0x789)]
    public fun test_try_cancel_order_with_client_order_id_wrong_user(
        user1: &signer, user2: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);
        let user2_addr = signer::address_of(user2);
        let client_order_id = 12345;
        let order_id = new_order_id_type(1);

        // Create an order request with client order ID for user1
        let order_req =
            new_order_request(
                user1_addr,
                order_id,
                option::some(client_order_id),
                1000, // price
                100, // orig_size
                100, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );

        // Place the maker order
        order_book.place_maker_order(order_req);

        // Test: Try to cancel order with correct client order ID but wrong user - should abort
        assert!(
            order_book.try_cancel_order_with_client_order_id(user2_addr, client_order_id).is_none()
        );
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_multiple_orders(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);

        // Create multiple orders with different client order IDs
        let client_order_id_1 = 1001;
        let client_order_id_2 = 1002;
        let client_order_id_3 = 1003;

        let order_id_1 = new_order_id_type(1);
        let order_id_2 = new_order_id_type(2);
        let order_id_3 = new_order_id_type(3);

        // Create and place first order
        let order_req_1 =
            new_order_request(
                user1_addr,
                order_id_1,
                option::some(client_order_id_1),
                1000, // price
                100, // orig_size
                100, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );
        order_book.place_maker_order(order_req_1);

        // Create and place second order
        let order_req_2 =
            new_order_request(
                user1_addr,
                order_id_2,
                option::some(client_order_id_2),
                1001, // price
                200, // orig_size
                200, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                43 // metadata
            );
        order_book.place_maker_order(order_req_2);

        // Create and place third order
        let order_req_3 =
            new_order_request(
                user1_addr,
                order_id_3,
                option::some(client_order_id_3),
                1002, // price
                300, // orig_size
                300, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                44 // metadata
            );
        order_book.place_maker_order(order_req_3);

        // Test: Cancel orders in different order than they were placed
        let cancel_result_2 =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id_2);
        assert!(cancel_result_2.is_some());

        let cancel_result_1 =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id_1);
        assert!(cancel_result_1.is_some());

        let cancel_result_3 =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id_3);
        assert!(cancel_result_3.is_some());

        // Test: Try to cancel already cancelled orders - should all return false
        let cancel_result_1_again =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id_1);
        assert!(cancel_result_1_again.is_none());

        let cancel_result_2_again =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id_2);
        assert!(cancel_result_2_again.is_none());

        let cancel_result_3_again =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id_3);
        assert!(cancel_result_3_again.is_none());
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_no_client_id(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);
        let order_id = new_order_id_type(1);

        // Create an order request WITHOUT client order ID
        let order_req =
            new_order_request(
                user1_addr,
                order_id,
                option::none(), // No client order ID
                1000, // price
                100, // orig_size
                100, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );

        // Place the maker order
        order_book.place_maker_order(order_req);

        // Test: Try to cancel with any client order ID - should return false
        let cancel_result =
            order_book.try_cancel_order_with_client_order_id(user1_addr, 12345);
        assert!(cancel_result.is_none());
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_fully_matched(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);
        let _user2_addr = @0x789;
        let client_order_id = 12345;

        // Create maker order (bid) with client order ID
        let maker_order_id = new_order_id_type(1);
        let maker_order_req =
            new_order_request(
                user1_addr,
                maker_order_id,
                option::some(client_order_id),
                1000, // price
                100, // orig_size
                100, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );
        order_book.place_maker_order(maker_order_req);

        // Verify order is in the book and can be cancelled before matching
        let cancel_result_before_match =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
        assert!(cancel_result_before_match.is_some());

        // Re-add the order for matching test
        let maker_order_id2 = new_order_id_type(2);
        let maker_order_req2 =
            new_order_request(
                user1_addr,
                maker_order_id2,
                option::some(client_order_id),
                1000, // price
                100, // orig_size
                100, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );
        order_book.place_maker_order(maker_order_req2);

        // Verify this is a taker order
        let is_taker = order_book.is_taker_order(1000, false, option::none());
        assert!(is_taker);

        // Execute the match - this should fully fill the maker order
        let single_match =
            order_book.get_single_match_for_taker(1000, 100, false);

        // Verify the match
        let matched_size = single_match.get_matched_size();
        assert!(matched_size == 100);

        // Now try to cancel the fully matched order - should return false
        // because the order was removed from both orders map and client_order_ids map
        let cancel_result_after_full_match =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
        assert!(cancel_result_after_full_match.is_none());
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_partially_matched(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);
        let client_order_id = 12345;

        // Create maker order (bid) with client order ID - larger size
        let maker_order_id = new_order_id_type(1);
        let maker_order_req =
            new_order_request(
                user1_addr,
                maker_order_id,
                option::some(client_order_id),
                1000, // price
                200, // orig_size (larger than what will be matched)
                200, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );
        order_book.place_maker_order(maker_order_req);

        // Verify this is a taker order
        let is_taker = order_book.is_taker_order(1000, false, option::none());
        assert!(is_taker);

        // Execute the match - this should partially fill the maker order
        let single_match =
            order_book.get_single_match_for_taker(1000, 100, false);

        // Verify the match
        let matched_size = single_match.get_matched_size();
        assert!(matched_size == 100);

        // Verify the remaining size of the maker order
        let remaining_size = order_book.get_remaining_size(maker_order_id);
        assert!(remaining_size == 100); // 200 - 100 = 100

        // Now try to cancel the partially matched order - should return true
        // because the order still exists in the order book with remaining size
        let cancel_result_after_partial_match =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
        assert!(cancel_result_after_partial_match.is_some());

        // Verify the order is now removed
        let remaining_size_after_cancel = order_book.get_remaining_size(maker_order_id);
        assert!(remaining_size_after_cancel == 0);

        // Try to cancel again - should return false
        let cancel_result_after_cancel =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
        assert!(cancel_result_after_cancel.is_none());
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_edge_cases(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);

        // Test with edge case client order IDs
        let edge_cases = vector[
            0, // Zero
            1, // Minimum positive
            18446744073709551615 // Maximum u64 value
        ];

        let i = 0;
        while (i < edge_cases.length()) {
            let client_order_id = edge_cases[i];
            let order_id = new_order_id_type((i as u128) + 1);

            // Create and place order
            let order_req =
                new_order_request(
                    user1_addr,
                    order_id,
                    option::some(client_order_id),
                    1000 + i, // Different prices
                    100, // orig_size
                    100, // remaining_size
                    true, // is_bid
                    option::none(), // trigger_condition
                    good_till_cancelled(),
                    42 // metadata
                );
            order_book.place_maker_order(order_req);

            // Test cancellation
            let cancel_result =
                order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
            assert!(cancel_result.is_some());

            i += 1;
        };
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_multiple_partial_matches(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);
        let client_order_id = 12345;

        // Create large maker order (bid) with client order ID
        let maker_order_id = new_order_id_type(1);
        let maker_order_req =
            new_order_request(
                user1_addr,
                maker_order_id,
                option::some(client_order_id),
                1000, // price
                300, // orig_size (will be matched in multiple parts)
                300, // remaining_size
                true, // is_bid
                option::none(), // trigger_condition
                good_till_cancelled(),
                42 // metadata
            );
        order_book.place_maker_order(maker_order_req);

        // First partial match
        let single_match1 =
            order_book.get_single_match_for_taker(1000, 100, false);
        assert!(single_match1.get_matched_size() == 100, 0);
        assert!(order_book.get_remaining_size(maker_order_id) == 200, 1);

        // Order should still be cancellable after first partial match
        let can_cancel_after_first =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id + 1); // Wrong client order ID
        assert!(can_cancel_after_first.is_none());

        let can_cancel_after_first_correct =
            order_book.try_cancel_order_with_client_order_id(user1_addr, client_order_id);
        assert!(can_cancel_after_first_correct.is_some());

        // Verify order is now removed after cancellation
        assert!(order_book.get_remaining_size(maker_order_id) == 0);
        destroy_order_book(order_book);
    }

    #[test(user1 = @0x456)]
    public fun test_try_cancel_order_with_client_order_id_sequential_full_matches(
        user1: &signer
    ) {
        // Setup a basic order book
        let order_book = set_up_test_with_id();
        let user1_addr = signer::address_of(user1);

        // Create multiple orders with different client order IDs that will be fully matched
        let client_order_ids = vector[1001, 1002, 1003];
        let order_ids = vector[new_order_id_type(1), new_order_id_type(2), new_order_id_type(3)];

        // Place all maker orders
        let i = 0;
        while (i < client_order_ids.length()) {
            let client_order_id = client_order_ids[i];
            let order_id = order_ids[i];

            let order_req =
                new_order_request(
                    user1_addr,
                    order_id,
                    option::some(client_order_id),
                    1000 + i, // Different prices
                    100, // orig_size
                    100, // remaining_size
                    true, // is_bid
                    option::none(), // trigger_condition
                    good_till_cancelled(),
                    42 // metadata
                );
            order_book.place_maker_order(order_req);
            i += 1;
        };

        // Fully match the first order
        let single_match1 =
            order_book.get_single_match_for_taker(1002, 100, false);
        assert!(single_match1.get_matched_size() == 100);

        // Try to cancel the fully matched order - should return false
        let cancel_result_fully_matched =
            order_book.try_cancel_order_with_client_order_id(user1_addr, 1003); // This order was fully matched
        assert!(cancel_result_fully_matched.is_none());

        // Try to cancel the remaining orders - should return true
        let cancel_result_1 =
            order_book.try_cancel_order_with_client_order_id(user1_addr, 1001);
        assert!(cancel_result_1.is_some());

        let cancel_result_2 =
            order_book.try_cancel_order_with_client_order_id(user1_addr, 1002);
        assert!(cancel_result_2.is_some());
        destroy_order_book(order_book);
    }
}
