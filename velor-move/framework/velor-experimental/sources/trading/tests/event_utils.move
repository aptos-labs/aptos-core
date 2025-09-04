#[test_only]
module velor_experimental::event_utils {
    use std::option::Option;
    use velor_framework::event;
    struct EventStore has drop {
        last_index: u64
    }

    public fun new_event_store(): EventStore {
        EventStore { last_index: 0 }
    }

    public fun latest_emitted_events<T: drop + copy + store>(
        store: &mut EventStore, limit: Option<u64>
    ): vector<T> {
        let events = event::emitted_events<T>();
        let end_index =
            if (limit.is_none()) {
                events.length()
            } else {
                let limit = limit.destroy_some();
                store.last_index + limit
            };
        let latest_events = events.slice(store.last_index, end_index);
        store.last_index = end_index;
        latest_events
    }
}
