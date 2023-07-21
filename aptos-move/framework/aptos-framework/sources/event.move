/// The Event module defines an `EventHandleGenerator` that is used to create
/// `EventHandle`s with unique GUIDs. It contains a counter for the number
/// of `EventHandle`s it generates. An `EventHandle` is used to count the number of
/// events emitted to a handle and emit events to the event store.
module aptos_framework::event {
    use std::bcs;

    use aptos_framework::guid::GUID;
    use aptos_framework::aggregator_v2::{Self, Aggregator, AggregatorSnapshot};

    friend aptos_framework::account;
    friend aptos_framework::object;

    const MAX_U64: u64 = 18446744073709551615;

    /// A handle for an event such that:
    /// 1. Other modules can emit events to this handle.
    /// 2. Storage can use this handle to prove the total number of events that happened in the past.
    struct EventHandle<phantom T: drop + store> has store {
        /// Total number of events emitted to this event stream.
        counter: u64,
        /// A globally unique ID for this event stream.
        guid: GUID,
    }

    struct ConcurrentEventHandle<phantom T: drop + store> has store {
        /// Total number of events emitted to this event stream.
        counter: Aggregator<u64>,
        /// A globally unique ID for this event stream.
        guid: GUID,
    }

    /// Use EventHandleGenerator to generate a unique event handle for `sig`
    public(friend) fun new_event_handle<T: drop + store>(guid: GUID): EventHandle<T> {
        EventHandle<T> {
            counter: 0,
            guid,
        }
    }

    public(friend) fun new_concurrent_event_handle<T: drop + store>(guid: GUID): ConcurrentEventHandle<T> {
        ConcurrentEventHandle<T> {
            counter: aggregator_v2::create_aggregator(MAX_U64),
            guid,
        }
    }

    /// Emit an event with payload `msg` by using `handle_ref`'s key and counter.
    public fun emit_event<T: drop + store>(handle_ref: &mut EventHandle<T>, msg: T) {
        write_to_event_store<T>(bcs::to_bytes(&handle_ref.guid), handle_ref.counter, msg);
        spec {
            assume handle_ref.counter + 1 <= MAX_U64;
        };
        handle_ref.counter = handle_ref.counter + 1;
    }

    /// Emit an event with payload `msg` by using `handle_ref`'s key and counter.
    public fun emit_concurrent_event<T: drop + store>(handle_ref: &mut ConcurrentEventHandle<T>, msg: T) {
        write_concurrent_to_event_store<T>(bcs::to_bytes(&handle_ref.guid), aggregator_v2::snapshot(&handle_ref.counter), msg);
        aggregator_v2::add(&mut handle_ref.counter, 1);
    }

    /// Return the GUID associated with this EventHandle
    public fun guid<T: drop + store>(handle_ref: &EventHandle<T>): &GUID {
        &handle_ref.guid
    }

    /// Return the current counter associated with this EventHandle
    public fun counter<T: drop + store>(handle_ref: &EventHandle<T>): u64 {
        handle_ref.counter
    }

    /// Log `msg` as the `count`th event associated with the event stream identified by `guid`
    native fun write_to_event_store<T: drop + store>(guid: vector<u8>, count: u64, msg: T);

    /// Log `msg` as the `count`th event associated with the event stream identified by `guid`
    native fun write_concurrent_to_event_store<T: drop + store>(guid: vector<u8>, count: AggregatorSnapshot<u64>, msg: T);

    /// Destroy a unique handle.
    public fun destroy_handle<T: drop + store>(handle: EventHandle<T>) {
        EventHandle<T> { counter: _, guid: _ } = handle;
    }

    /// Destroy a unique handle.
    public fun destroy_concurrent_handle<T: drop + store>(handle: ConcurrentEventHandle<T>) {
        ConcurrentEventHandle<T> { counter: _, guid: _ } = handle;
    }

    #[test_only]
    public native fun emitted_events_by_handle<T: drop + store>(handle: &EventHandle<T>): vector<T>;

    #[test_only]
    public fun was_event_emitted_by_handle<T: drop + store>(handle: &EventHandle<T>, msg: &T): bool {
        use std::vector;
        vector::contains(&emitted_events_by_handle(handle), msg)
    }
}
