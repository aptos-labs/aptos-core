spec aptos_experimental::order_book_utils {

    spec new_default_big_ordered_map<K: store, V: store>()
        : 0x1::big_ordered_map::BigOrderedMap<K, V> {
        use 0x1::big_ordered_map;
        pragma opaque = true;
        // `new_with_config` is an intrinsic map constructor. Its result carries
        // a fresh hidden iterator-validity token, so equality with a separate
        // `result_of` call is deliberately not meaningful. The intrinsic map
        // model exposes its complete observable result: an empty key set.
        ensures [inferred] big_ordered_map::spec_len(result) == 0;
        ensures [inferred] forall k: K: !big_ordered_map::spec_contains_key(result, k);
        aborts_if [inferred] false;
    }
}
