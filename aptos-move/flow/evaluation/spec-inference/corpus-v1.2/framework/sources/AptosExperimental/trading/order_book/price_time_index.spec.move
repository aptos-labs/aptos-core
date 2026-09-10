spec aptos_experimental::price_time_index {
    use aptos_framework::big_ordered_map;

    /// Both sides start as empty maps under the default configuration, whose
    /// degrees are within the range the map accepts.
    spec new_price_time_idx(): PriceTimeIndex {
        pragma opaque;
        aborts_if false;
        ensures result is PriceTimeIndex::V1;
        ensures big_ordered_map::spec_len(result.buys) == 0;
        ensures big_ordered_map::spec_len(result.sells) == 0;
    }
}
