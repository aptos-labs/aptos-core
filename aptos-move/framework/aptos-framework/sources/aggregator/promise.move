module aptos_framework::promise {
    use std::option::{Self, Option};

    /// The error code raised when `get_value` function is called before
    /// resolving the promise.
    const EPROMISE_NOT_RESOLVED: u64 = 1;

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

    public fun get_value(promise: &Promise): u128 {
        assert!(option::is_none(&promise.id), EPROMISE_NOT_RESOLVED);
        promise.value
    }
}