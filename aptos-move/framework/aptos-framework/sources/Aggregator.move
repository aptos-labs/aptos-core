module AptosFramework::Aggregator {
    use Std::Errors;
    use Std::Option;

    use AptosFramework::IterableTable::{Self, IterableTable};

    //
    // Errors.
    //

    /// When `Aggregator` is destroyed but still holds values.
    const EAGGREGATOR_NOT_EMPTY: u64 = 0;

    /// When `Aggregator` overflows on adding a new value.
    const EAGGREGATOR_OVERFLOW: u64 = 1;

    //
    // Core data structures.
    //

    const MAX_U128: u128 = 340282366920938463463374607431768211455;

    /// Main structure representing an aggregator.
    struct Aggregator has store {
        buckets: IterableTable<u64, u128>
    }

    /// Returns a new empty aggregator instance.
    public fun new(): Aggregator {
       Aggregator { buckets: IterableTable::new() }
    }

    /// Returns true if the aggregator instance contains no values.
    public fun empty(aggregator: & Aggregator): bool {
       IterableTable::empty(&aggregator.buckets)
    }

    /// Destroys given aggregator instance.
    public fun destroy_empty(aggregator: Aggregator) {
        assert!(IterableTable::empty(&aggregator.buckets), Errors::invalid_argument(EAGGREGATOR_NOT_EMPTY));
        let Aggregator { buckets } = aggregator;
        IterableTable::destroy_empty(buckets);
    }

    /// Adds a new value to the aggregator by storing it in one of the buckets.
    public fun add(aggregator: &mut Aggregator, v: u64) {
        let idx = get_bucket();
        let value = (v as u128);

        if (IterableTable::contains(&aggregator.buckets, idx)) {
            let amount = IterableTable::borrow_mut(&mut aggregator.buckets, idx);
            assert!(*amount <= MAX_U128 - value, Errors::invalid_argument(EAGGREGATOR_OVERFLOW));
            *amount = *amount + value;
        } else {
            IterableTable::add(&mut aggregator.buckets, idx, value);
        } 
    }

    /// Returns the sum of all values aggregator instance currently stores and removes all of them.
    public fun drain(aggregator: &mut Aggregator): u128 {
        let amount = 0;

        let key = IterableTable::head_key(&aggregator.buckets);
        while (Option::is_some(&key)) {
            let (value, _, next) = IterableTable::remove_iter(&mut aggregator.buckets, *Option::borrow(&key));
            key = next;
            assert!(amount <= MAX_U128 - value, Errors::invalid_argument(EAGGREGATOR_OVERFLOW));
            amount = amount + value;
        };
        amount
    }

    /// Returns an index into the bucket based on transaction context.
    native fun get_bucket(): u64;

    //
    // Tests
    //

    #[test]
    fun aggregator_test() {
        let agg = new();

        add(&mut agg, 10);
        add(&mut agg, 10);
        add(&mut agg, 11);
        assert!(drain(&mut agg) == 31, 0);

        add(&mut agg, 100);
        assert!(drain(&mut agg) == 100, 0);

        add(&mut agg, 1);
        assert!(drain(&mut agg) == 1, 0);

        destroy_empty(agg);
    }
}
