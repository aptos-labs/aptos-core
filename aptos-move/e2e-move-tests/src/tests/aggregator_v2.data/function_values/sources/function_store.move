module 0x1::function_store {
    use aptos_framework::aggregator_v2::{Aggregator, create_unbounded_aggregator};

    struct FunctionStore has key, store {
        // Capturing aggregators, snapshots or anything that contains delayed fields is not
        // allowed. This is enforced at runtime (serialization-time).
        //
        // Still, it is possible to define a resource that may try to capture the aggregator.
        // Because the aggregator is not copy, we cannot have a copyable closure capturing it.
        // Nevertheless, it is possible to have a non-copy closure that captures an aggregator
        // that can be updated by moving the resource from and back to the same address.
        apply: |u64|u64 has store,
    }

    public fun fetch_and_add(aggregator: Aggregator<u64>, value: u64): u64 {
        aggregator.try_add(value);
        aggregator.read()
    }

    public entry fun try_initialize_should_abort(account: &signer, value: u64) {
        let aggregator = create_unbounded_aggregator<u64>();
        aggregator.add(value);

        let apply = |x| fetch_and_add(aggregator, x);
        move_to(account, FunctionStore { apply });
    }

    public entry fun function_store_does_not_exist(account: &signer) {
        let addr = std::signer::address_of(account);
        let exists = exists<FunctionStore>(addr);
        assert!(!exists, 777);
    }

    #[view]
    public fun view_function_store_exists(addr: address): bool {
        exists<FunctionStore>(addr)
    }
}
