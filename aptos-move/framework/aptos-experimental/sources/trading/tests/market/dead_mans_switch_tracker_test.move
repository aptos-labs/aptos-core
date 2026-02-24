#[test_only]
module aptos_experimental::dead_mans_switch_tracker_test {
    use std::option;
    use std::signer;
    use aptos_framework::timestamp;
    use aptos_experimental::dead_mans_switch_tracker::{
        new_dead_mans_switch_tracker_for_test,
        is_order_valid,
        update_keep_alive_state_for_test,
        disable_keep_alive_for_test,
        destroy_tracker,
        DeadMansSwitchTracker
    };

    const MIN_KEEP_ALIVE_TIME_SECS: u64 = 10; // 10 seconds minimum
    const INITIAL_TIMESTAMP: u64 = 1000;

    // Test utility functions
    fun setup_test(aptos_framework: &signer): DeadMansSwitchTracker {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(INITIAL_TIMESTAMP);
        new_dead_mans_switch_tracker_for_test(MIN_KEEP_ALIVE_TIME_SECS)
    }

    fun set_time(time_secs: u64) {
        timestamp::update_global_time_for_test_secs(time_secs)
    }

    fun update_keep_alive(
        tracker: &mut DeadMansSwitchTracker, user_addr: address, timeout: u64
    ) {
        update_keep_alive_state_for_test(tracker, user_addr, timeout)
    }

    fun disable_keep_alive(
        tracker: &mut DeadMansSwitchTracker, user_addr: address
    ) {
        disable_keep_alive_for_test(tracker, user_addr)
    }

    fun assert_order_valid(
        tracker: &DeadMansSwitchTracker, user_addr: address, order_time: u64
    ) {
        assert!(
            is_order_valid(tracker, user_addr, option::some(order_time)),
            0
        )
    }

    fun assert_order_invalid(
        tracker: &DeadMansSwitchTracker, user_addr: address, order_time: u64
    ) {
        assert!(
            !is_order_valid(tracker, user_addr, option::some(order_time)),
            0
        )
    }

    #[test(aptos_framework = @0x1)]
    public fun test_new_tracker_allows_all_orders(
        aptos_framework: &signer
    ) {
        let tracker = setup_test(aptos_framework);
        let user_addr = @0x123;

        // When no keep-alive is set, all orders should be valid
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP - 100);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP + 100);

        destroy_tracker(tracker)
    }

    #[test(aptos_framework = @0x1, user = @0x123)]
    public fun test_keep_alive_update(
        aptos_framework: &signer, user: &signer
    ) {
        let tracker = setup_test(aptos_framework);
        let user_addr = signer::address_of(user);
        let timeout = 60;

        // Update keep-alive for the first time
        update_keep_alive(&mut tracker, user_addr, timeout);

        // All past orders should be valid (session_start_time is 0 for first update)
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP - 500);
        // Future orders within timeout should be valid
        set_time(INITIAL_TIMESTAMP + 30);
        update_keep_alive(&mut tracker, user_addr, timeout);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP + 30);

        destroy_tracker(tracker)
    }

    #[test(aptos_framework = @0x1, user = @0x123)]
    public fun test_keep_alive_expiration_and_new_session(
        aptos_framework: &signer, user: &signer
    ) {
        let tracker = setup_test(aptos_framework);
        let user_addr = signer::address_of(user);
        let timeout = 60;

        update_keep_alive(&mut tracker, user_addr, timeout);

        // Move time forward but still within timeout
        set_time(INITIAL_TIMESTAMP + 59);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP);

        // Move time forward past expiration
        set_time(INITIAL_TIMESTAMP + 120);
        assert_order_invalid(&tracker, user_addr, INITIAL_TIMESTAMP);

        update_keep_alive(&mut tracker, user_addr, timeout);

        // After updating keep-alive, old orders should still be invalid
        assert_order_invalid(&tracker, user_addr, INITIAL_TIMESTAMP + 60);
        // New orders after the update should be valid
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP + 130);

        destroy_tracker(tracker)
    }

    #[test(aptos_framework = @0x1, user = @0x123)]
    public fun test_zero_timeout_disables_keep_alive(
        aptos_framework: &signer, user: &signer
    ) {
        let tracker = setup_test(aptos_framework);
        let user_addr = signer::address_of(user);

        // Set keep-alive
        update_keep_alive(&mut tracker, user_addr, 60);

        // Disable by setting timeout to 0
        update_keep_alive(&mut tracker, user_addr, 0);

        // All orders should now be valid
        set_time(INITIAL_TIMESTAMP + 1000);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP + 500);

        destroy_tracker(tracker)
    }

    #[test(aptos_framework = @0x1, user1 = @0x123, user2 = @0x456)]
    public fun test_multiple_users_independent(
        aptos_framework: &signer, user1: &signer, user2: &signer
    ) {
        let tracker = setup_test(aptos_framework);
        let user1_addr = signer::address_of(user1);
        let user2_addr = signer::address_of(user2);

        // User1 sets keep-alive
        update_keep_alive(&mut tracker, user1_addr, 60);

        // User2 has no keep-alive, so all orders valid
        assert_order_valid(&tracker, user2_addr, INITIAL_TIMESTAMP);

        // User1's orders expire after timeout
        set_time(INITIAL_TIMESTAMP + 70);
        assert_order_invalid(&tracker, user1_addr, INITIAL_TIMESTAMP + 10);

        // User2 still valid (no keep-alive)
        assert_order_valid(&tracker, user2_addr, INITIAL_TIMESTAMP);

        destroy_tracker(tracker)
    }

    #[test(aptos_framework = @0x1, user = @0x123)]
    #[
        expected_failure(
            abort_code = 0, location = aptos_experimental::dead_mans_switch_tracker
        )
    ]
    public fun test_timeout_too_short_fails(
        aptos_framework: &signer, user: &signer
    ) {
        let tracker = setup_test(aptos_framework);
        let user_addr = signer::address_of(user);

        // Try to set timeout less than minimum
        update_keep_alive(&mut tracker, user_addr, MIN_KEEP_ALIVE_TIME_SECS - 1);

        destroy_tracker(tracker)
    }

    #[test(aptos_framework = @0x1, user = @0x123)]
    public fun test_exact_expiration_boundary(
        aptos_framework: &signer, user: &signer
    ) {
        let tracker = setup_test(aptos_framework);
        let user_addr = signer::address_of(user);
        let timeout = 100;

        update_keep_alive(&mut tracker, user_addr, timeout);

        // One second before expiration
        set_time(INITIAL_TIMESTAMP + 99);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP);

        // At exact expiration time (expiration_time >= current_time is true, so still valid)
        set_time(INITIAL_TIMESTAMP + 100);
        assert_order_valid(&tracker, user_addr, INITIAL_TIMESTAMP);

        // One second after expiration (now expired)
        set_time(INITIAL_TIMESTAMP + 101);
        assert_order_invalid(&tracker, user_addr, INITIAL_TIMESTAMP);

        destroy_tracker(tracker)
    }
}
