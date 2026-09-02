spec aptos_experimental::dead_mans_switch_tracker {

    spec disable_keep_alive(
        tracker: &mut 0x7::dead_mans_switch_tracker::DeadMansSwitchTracker,
        parent: address,
        market: address,
        account: address
    ) {
        use 0x1::event;
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] tracker
            == update_field(
                old(tracker),
                state,
                big_ordered_map::spec_remove(old(tracker).state, account)
            );
        ensures [inferred] ensures_of<event::emit<KeepAliveDisabledEvent>> (
            KeepAliveDisabledEvent::V1 {
                parent: parent,
                market: market,
                account: account,
                was_registered: big_ordered_map::spec_contains_key(
                    old(tracker).state, account
                )
            }
        );
        aborts_if [inferred] aborts_of<event::emit<KeepAliveDisabledEvent>> (
            KeepAliveDisabledEvent::V1 {
                parent: parent,
                market: market,
                account: account,
                was_registered: big_ordered_map::spec_contains_key(tracker.state, account)
            }
        );
    }

    spec new_dead_mans_switch_tracker(
        min_keep_alive_time_secs: u64
    ): 0x7::dead_mans_switch_tracker::DeadMansSwitchTracker {
        use 0x7::order_book_utils;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] result
            == DeadMansSwitchTracker {
                min_keep_alive_time_secs: min_keep_alive_time_secs,
                state: result_of<order_book_utils::new_default_big_ordered_map<address,
                KeepAliveState>> ()
            };
    }

    spec set_min_keep_alive_time_secs(
        tracker: &mut 0x7::dead_mans_switch_tracker::DeadMansSwitchTracker,
        parent: address,
        market: address,
        min_keep_alive_time_secs: u64
    ) {
        use 0x1::event;
        pragma opaque = true;
        ensures [inferred] ensures_of<event::emit<MinKeepAliveTimeUpdatedEvent>> (
            MinKeepAliveTimeUpdatedEvent::V1 {
                parent: parent,
                market: market,
                old_min_keep_alive_time_secs: old(tracker).min_keep_alive_time_secs,
                new_min_keep_alive_time_secs: min_keep_alive_time_secs
            }
        );
        ensures [inferred] tracker
            == update_field(
                old(tracker), min_keep_alive_time_secs, min_keep_alive_time_secs
            );
        aborts_if [inferred] aborts_of<event::emit<MinKeepAliveTimeUpdatedEvent>> (
            MinKeepAliveTimeUpdatedEvent::V1 {
                parent: parent,
                market: market,
                old_min_keep_alive_time_secs: tracker.min_keep_alive_time_secs,
                new_min_keep_alive_time_secs: min_keep_alive_time_secs
            }
        );
    }
}
