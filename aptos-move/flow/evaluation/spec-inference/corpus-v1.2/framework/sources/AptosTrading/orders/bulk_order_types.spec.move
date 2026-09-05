spec aptos_trading::bulk_order_types {
    /// Sum of the sizes from `i` on, as unbounded arithmetic.
    spec fun spec_sum_from(sizes: vector<u64>, i: u64): num [weight = 20] {
        if (i >= len(sizes)) 0 else sizes[i] + spec_sum_from(sizes, i + 1)
    }

    spec fun spec_side_sizes<M>(request: BulkOrderRequest<M>, is_bid: bool): vector<u64> {
        if (is_bid) request.bid_sizes else request.ask_sizes
    }

    spec module {
        /// A sum of unsigned sizes is not negative.
        lemma sum_from_nonneg(sizes: vector<u64>, i: u64) {
            requires i <= len(sizes);
            ensures spec_sum_from(sizes, i) >= 0;
            decreases len(sizes) - i;
        } proof {
            if (i < len(sizes)) {
                apply sum_from_nonneg(sizes, i + 1);
            }
        }
    }

    /// The sizes are summed with `u64` addition, which aborts on overflow;
    /// partial sums never decrease, so the total decides.
    spec get_total_remaining_size<M: store + copy + drop>(self: &BulkOrderRequest<M>, is_bid: bool): u64 {
        pragma opaque;
        let sizes = spec_side_sizes(self, is_bid);
        aborts_if spec_sum_from(sizes, 0) > MAX_U64;
        ensures result == spec_sum_from(sizes, 0);
    } proof {
        forall i: u64 {spec_sum_from(sizes, i)} [weight = 20] apply sum_from_nonneg(sizes, i);
    }
}
