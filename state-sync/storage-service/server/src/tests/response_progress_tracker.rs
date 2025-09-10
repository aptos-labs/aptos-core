// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::storage::ResponseDataProgressTracker;
use aptos_time_service::TimeService;

#[test]
fn data_items_fits_in_response() {
    // Create a response progress tracker
    let max_response_size = 2000;
    let time_service = TimeService::mock();
    let mut response_progress_tracker = ResponseDataProgressTracker::new(
        100, // Number of items to fetch
        max_response_size,
        5000, // Max storage read wait time
        time_service.clone(),
    );

    // Verify that an item can only overflow the response size if "always_allow_first_item" is true
    let large_data_item_size = max_response_size * 2;
    assert!(!response_progress_tracker.data_items_fits_in_response(false, large_data_item_size));
    assert!(response_progress_tracker.data_items_fits_in_response(true, large_data_item_size));

    // Add a small item to the response
    let small_data_item_size = 1;
    assert!(response_progress_tracker.data_items_fits_in_response(false, small_data_item_size));
    response_progress_tracker.add_data_item(small_data_item_size);

    // Verify that the response tracker will not overflow the response size (even if the flag is true)
    assert!(!response_progress_tracker.data_items_fits_in_response(false, large_data_item_size));
    assert!(!response_progress_tracker.data_items_fits_in_response(true, large_data_item_size));

    // Add several medium items that fit into the response (adds 1600 bytes in total, total will be 1601)
    let medium_data_item_size = 400;
    for _ in 0..4 {
        assert!(response_progress_tracker.data_items_fits_in_response(false, medium_data_item_size));
        response_progress_tracker.add_data_item(medium_data_item_size);
    }

    // Verify that the response tracker will not overflow the response size
    assert!(!response_progress_tracker.data_items_fits_in_response(false, large_data_item_size));
    assert!(!response_progress_tracker.data_items_fits_in_response(false, medium_data_item_size));
    assert!(response_progress_tracker.data_items_fits_in_response(false, small_data_item_size));

    // Verify that we can add another item that just fits into the response (i.e., the total size is < max_response_size)
    let just_fits_data_item_size = 398; // 1601 + 398 = 1999 < 2000
    assert!(response_progress_tracker.data_items_fits_in_response(false, just_fits_data_item_size));
    response_progress_tracker.add_data_item(just_fits_data_item_size);

    // Verify that adding another small item (that would overflow the response size) is not allowed
    assert!(!response_progress_tracker.data_items_fits_in_response(false, small_data_item_size));
}

#[test]
fn is_response_complete_all_items_fetched() {
    // Create a response progress tracker
    let num_items_to_fetch = 100;
    let time_service = TimeService::mock();
    let mut response_progress_tracker = ResponseDataProgressTracker::new(
        num_items_to_fetch,
        2000, // Max response size
        5000, // Max storage read wait time
        time_service.clone(),
    );

    // Verify that the response is not complete
    assert!(!response_progress_tracker.is_response_complete());

    // Add almost all items to the response tracker (but not enough to complete)
    for _ in 0..num_items_to_fetch - 1 {
        response_progress_tracker.add_data_item(10);
    }

    // Verify that the response is not complete (still one item left to fetch)
    assert!(!response_progress_tracker.is_response_complete());

    // Add the last item to the response tracker
    response_progress_tracker.add_data_item(10);

    // Verify that the response is complete (all items were fetched)
    assert!(response_progress_tracker.is_response_complete());

    // Continue to add items to the response tracker (this should not affect the completion status)
    for _ in 0..10 {
        response_progress_tracker.add_data_item(10);
        assert!(response_progress_tracker.is_response_complete());
    }
}

#[test]
fn is_response_complete_overflowed_data_size() {
    // Create a response progress tracker
    let max_response_size = 2000;
    let time_service = TimeService::mock();
    let mut response_progress_tracker = ResponseDataProgressTracker::new(
        100, // Number of items to fetch
        max_response_size,
        5000, // Max storage read wait time
        time_service.clone(),
    );

    // Verify that the response is not complete
    assert!(!response_progress_tracker.is_response_complete());

    // Add a small item to the response tracker (not enough to overflow the size)
    response_progress_tracker.add_data_item(10);

    // Verify that the response is not complete
    assert!(!response_progress_tracker.is_response_complete());

    // Add a large item to the response tracker (but not enough to overflow the size)
    response_progress_tracker.add_data_item(max_response_size / 2);

    // Verify that the response is still not complete
    assert!(!response_progress_tracker.is_response_complete());

    // Add another large item to exceed the max response size
    response_progress_tracker.add_data_item(max_response_size / 2);

    // Verify that the response is complete (we overflowed the data size)
    assert!(response_progress_tracker.is_response_complete());
}

#[test]
fn is_response_complete_overflowed_storage_read_duration() {
    // Create a response progress tracker
    let max_storage_read_wait_time_ms = 5000;
    let time_service = TimeService::mock();
    let response_progress_tracker = ResponseDataProgressTracker::new(
        100,  // Number of items to fetch
        2000, // Max response size
        max_storage_read_wait_time_ms,
        time_service.clone(),
    );

    // Verify that the response is not complete
    assert!(!response_progress_tracker.is_response_complete());

    // Elapse some time (but not enough to overflow the storage read duration)
    let mock_time_service = time_service.into_mock();
    mock_time_service.advance_ms(max_storage_read_wait_time_ms / 2);

    // Verify that the response is still not complete
    assert!(!response_progress_tracker.is_response_complete());

    // Elapse more time to exceed the max storage read wait time
    mock_time_service.advance_ms(max_storage_read_wait_time_ms);

    // Verify that the response is complete (we overflowed the storage read duration)
    assert!(response_progress_tracker.is_response_complete());
}
