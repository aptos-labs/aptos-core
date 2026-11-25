//# publish
module 0xc0ffee::m {
    use std::bcs;
    use std::cmp;
    use std::mem;
    use std::vector;

    public fun move_range_between(): (vector<u64>, vector<u64>) {
        let from = vector[1, 2, 3, 4, 5];
        let to = vector[10, 20];
        vector::move_range(&mut from, 1, 3, &mut to, 1);
        (from, to)
    }

    public fun move_range_out_of_bounds() {
        let from = vector[1, 2];
        let to = vector[];
        vector::move_range(&mut from, 1, 2, &mut to, 0);
    }

    public fun swap_and_replace(): (u64, u64, u64) {
        let left = 1;
        let right = 2;
        mem::swap(&mut left, &mut right);
        let old = mem::replace(&mut left, 7);
        (left, right, old)
    }

    public fun serialized_size_matches(): bool {
        let value = vector[vector[1u8, 2], vector[]];
        bcs::serialized_size(&value) == vector::length(&bcs::to_bytes(&value))
    }

    public fun compare_orders(): (bool, bool) {
        let less = cmp::compare(&1u64, &2);
        let greater = cmp::compare(&2u64, &1);
        (less.is_lt(), greater.is_lt())
    }
}

//# run 0xc0ffee::m::move_range_between

//# run 0xc0ffee::m::move_range_out_of_bounds

//# run 0xc0ffee::m::swap_and_replace

//# run 0xc0ffee::m::serialized_size_matches

//# run 0xc0ffee::m::compare_orders
