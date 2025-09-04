module 0x1::proxy {
    use velor_framework::aggregator_v2::{Self, Aggregator};

    struct Counter has key, store, drop {
        aggregator: Aggregator<u64>,
    }

    public entry fun initialize(account: &signer, value: u64) {
        let aggregator = aggregator_v2::create_unbounded_aggregator<u64>();
        aggregator.add(value);
        move_to(account, Counter { aggregator });
    }

    public fun destroy(account: &signer): Aggregator<u64> acquires Counter {
        let addr = std::signer::address_of(account);
        let counter = move_from<Counter>(addr);
        let Counter { aggregator } = counter;
        aggregator
    }

    public fun add(self: &mut Counter, value: u64) {
        self.aggregator.add(value);
    }

    public fun borrow_and_add(addr: address, value: u64) acquires Counter {
        Counter[addr].add(value);
    }

    public fun move_roundtrip_with_action(account: &signer, action: |Counter|Counter) acquires Counter {
        let addr = std::signer::address_of(account);
        let counter = move_from<Counter>(addr);
        let counter = action(counter);
        move_to(account, counter);
    }
}
