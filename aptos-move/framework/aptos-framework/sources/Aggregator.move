module AptosFramework::Aggregator {
    use Std::Errors;
    use Std::Option;

    use AptosFramework::IterableTable::{Self, IterableTable};
    
    const EAGGREGATOR_NOT_EMPTY: u64 = 0;

    struct Aggregator has store {
        buckets: IterableTable<u64, u64>
    }

    public fun new(): Aggregator {
       Aggregator { buckets: IterableTable::new() }
    }

    public fun empty(aggregator: & Aggregator): bool {
       IterableTable::empty(&aggregator.buckets)
    }

    public fun destroy_empty(aggregator: Aggregator) {
        assert!(IterableTable::empty(&aggregator.buckets), Errors::invalid_argument(EAGGREGATOR_NOT_EMPTY));
        let Aggregator { buckets } = aggregator;
        IterableTable::destroy_empty(buckets);
    }

    public fun add(aggregator: &mut Aggregator, value: u64) {
        let idx = get_bucket();

        if (IterableTable::contains(&aggregator.buckets, idx)) {
            let amount = IterableTable::borrow_mut(&mut aggregator.buckets, idx);
            *amount = *amount + value;
        } else {
            IterableTable::add(&mut aggregator.buckets, idx, value);
        } 
    }

    public fun drain(aggregator: &mut Aggregator): u64 {
        let amount = 0;

        let key = IterableTable::head_key(&aggregator.buckets);
        while (Option::is_some(&key)) {
            let (value, _, next) = IterableTable::remove_iter(&mut aggregator.buckets, *Option::borrow(&key));
            key = next;
            amount = amount + value;
        };
        amount
    }

    native public fun get_bucket(): u64;

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
