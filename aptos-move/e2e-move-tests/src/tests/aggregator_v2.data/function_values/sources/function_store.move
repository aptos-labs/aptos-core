module 0x1::function_store {
    use aptos_framework::aggregator_v2::{Aggregator, create_unbounded_aggregator};

    struct FunctionStore has key, store {
        // Note: aggregator is not copy, so we cannot have a copyable closure here. Still, it is
        // possible to define a resource that may try to capture the aggregator (e.g., if the
        // resource is moved from and back to the same address, we can bypass the copy limitation).
        //
        // In any case, caturing aggregators is not allowed. It is only checked at serialization
        // time.
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
