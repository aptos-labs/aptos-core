module aptos_framework::promise {
    use std::option::{Self, Option};
    use aptos_framework::aggregator::{Self, Aggregator};

    struct Promise has store {
        value: u128,
        id: Option<address>
    }

    // TODO: Do we even need a new function?
    public fun new(id: address): Promise {
        Promise {
            value: 0,
            id: option::some(id)
        }
    }

    public fun get_value(promise: &mut Promise): u128 {
        if (option::is_some(&promise.id)) {
            let id = option::extract(&mut promise.id);
            promise.value = aggregator::read(id);
        };
        promise.value
    }
}