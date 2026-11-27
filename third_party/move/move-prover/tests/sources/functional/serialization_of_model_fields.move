// Serialization of a declaration the verification model enriches: the
// frontend appends `val` to `0x1::aggregator::Aggregator` after its physical
// fields `handle`, `key` and `limit`. BCS serializes the physical fields only,
// so equal serializations give those and nothing about `val`.
module 0x1::aggregator {
    use std::bcs;

    struct Aggregator has store { handle: address, key: address, limit: u128 }

    spec native fun spec_get_limit(a: Aggregator): u128;
    spec native fun spec_aggregator_get_val(a: Aggregator): u128;

    fun same_value(a: &Aggregator, b: &Aggregator): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec same_value {
        ensures result ==> spec_aggregator_get_val(a) == spec_aggregator_get_val(b); // error: the model field is not serialized
    }

    fun same_value_spec_only(a: &Aggregator, b: &Aggregator): bool { true }
    spec same_value_spec_only {
        ensures bcs::serialize(a) == bcs::serialize(b)
            ==> spec_aggregator_get_val(a) == spec_aggregator_get_val(b); // error: the model field is not serialized
    }

    fun same_limit(a: &Aggregator, b: &Aggregator): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec same_limit {
        ensures result ==> spec_get_limit(a) == spec_get_limit(b);
    }

    fun same_handle(a: &Aggregator, b: &Aggregator): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec same_handle {
        ensures result ==> a.handle == b.handle && a.key == b.key;
    }
}
