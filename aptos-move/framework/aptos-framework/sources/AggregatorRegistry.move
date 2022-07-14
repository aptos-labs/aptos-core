module AptosFramework::Aggregator {
    use Std::Signer;
    use AptosFramework::Table::{Self, Table};

    // ======================== Aggregator registry ======================== //

    /// Aggregator registry  has already been published. 
    const E_AGGREGATOR_REGISTRY_ALREADY_EXISTS: u64 = 1500;

    /// A global map of all registered aggregators as
    /// (aggregator_key, agregator_value) pairs. Values access is restricted
    /// and only `Aggregator` associated with a key can read or update the
    /// value.
    struct AggregatorRegistry has key {
        table: Table<u128, u128>,
    }

    /// Pusblishes a new registry for aggregators in the `account`.
    public fun new_aggregator_registry(account: &signer) {
        let addr = Signer::address_of(account);
        assert!(!exists<AggregatorRegistry>(addr), E_AGGREGATOR_REGISTRY_ALREADY_EXISTS);
        
        let registry = AggregatorRegistry {
            table: Table::new()
        };
        move_to(account, registry);
    }

    // ============================ Aggregator ============================= //

    /// If aggregator's value (actual or accumulated) overflows, raised by
    /// native code.
    const E_AGGREGATOR_OVERFLOW: u64 = 1600;

    /// If resolving aggregator's value failed.
    const E_AGGREGATOR_RESOLVE_FAILURE: u64 = 1601;

    /// If `Aggregator` cannot find its value when resolving from storage.
    const E_AGGREGATOR_VALUE_NOT_FOUND: u64 = 1602;

    /// Aggregator struct that can be uniquely identified by a key, internally
    /// stores an opaque value, initialized to 0, and overflowing on exceeding
    /// `limit`.
    struct Aggregator has store {
        table_handle: u128,
        key: u128,
        limit: u128,
    }

    /// Creates a new aggregator instance associated with `registry` and which
    /// overflows on exceeding `limit`. 
    public native fun new(registry: &mut AggregatorRegistry, limit: u128): Aggregator;

    /// Adds `value` to aggregator. Aborts on overflow.
    public native fun add(aggregator: &mut Aggregator, value: u128);

    /// Returns a value stored in this aggregator.
    public native fun read(aggregator: &Aggregator): u128;

    // ========================= Aggregator tests ========================== //

    #[test(account = @0xFF)]
    #[expected_failure(abort_code = 1500)]
    fun test_multiple_registries(account: signer) {
        new_aggregator_registry(&account);
        new_aggregator_registry(&account);
    }

    #[test_only]
    // For now destroy aggregators for tests only, but we actually need to
    // remove the resource from the `Table` stored under (table_handle, key).
    fun destroy(aggregator: Aggregator) {
        let Aggregator { table_handle: _, key: _, limit: _ } = aggregator;
    }

    #[test(account = @0xFF)]
    fun test_can_add_and_read(account: signer) acquires AggregatorRegistry {
        new_aggregator_registry(&account);
        let registry = borrow_global_mut<AggregatorRegistry>(Signer::address_of(&account));

        let aggregator = new(registry, /*limit=*/1000);

        add(&mut aggregator, 12);
        assert!(read(&aggregator) == 12, 0);

        add(&mut aggregator, 3);
        assert!(read(&aggregator) == 15, 0);

        destroy(aggregator);
    }

    #[test(account = @0xFF)]
    #[expected_failure(abort_code = 1600)]
    fun test_overflow(account: signer) acquires AggregatorRegistry {
        new_aggregator_registry(&account);
        let registry = borrow_global_mut<AggregatorRegistry>(Signer::address_of(&account));

        let aggregator = new(registry, /*limit=*/10);

        add(&mut aggregator, 12);
        destroy(aggregator);
    }
}
