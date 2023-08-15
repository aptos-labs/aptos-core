spec aptos_std::smart_vector {

    spec SmartVector {
        // `bucket_size` shouldn't be 0, if specified.
        invariant option::is_none(bucket_size)
            || (option::is_some(bucket_size) && option::borrow(bucket_size) != 0);
        // vector length should be <= `inline_capacity`, if specified.
        invariant option::is_none(inline_capacity)
            || (vector::length(inline_vec) <= option::borrow(inline_capacity));
        // both `inline_capacity` and `bucket_size` should either exist or shouldn't exist at all.
        invariant (option::is_none(inline_capacity) && option::is_none(bucket_size))
            || (option::is_some(inline_capacity) && option::is_some(bucket_size));
    }

    spec empty {
        aborts_if false;
    }

    spec empty_with_config {
        aborts_if bucket_size == 0;
    }

    spec destroy_empty {
        aborts_if !(is_empty(v));
        aborts_if vector::length(v.inline_vec) != 0
            || option::is_some(v.big_vec);
    }

    spec borrow {
        aborts_if i >= length(v);
        aborts_if option::is_some(v.big_vec) && (
            (vector::length(v.inline_vec) + big_vector::length<T>(option::borrow(v.big_vec))) > MAX_U64
        );
    }

    spec push_back<T: store>(v: &mut SmartVector<T>, val: T) {
        pragma verify = false; // TODO: set to false because of timeout
    }

    spec pop_back {
        use aptos_std::table_with_length;

        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)

        aborts_if  option::is_some(v.big_vec)
            &&
            (table_with_length::spec_len(option::borrow(v.big_vec).buckets) == 0);
        aborts_if is_empty(v);
        aborts_if option::is_some(v.big_vec) && (
            (vector::length(v.inline_vec) + big_vector::length<T>(option::borrow(v.big_vec))) > MAX_U64
        );

        ensures length(v) == length(old(v)) - 1;
    }

    spec swap_remove {
        pragma verify = false; // TODO: set because of timeout
        aborts_if i >= length(v);
        aborts_if option::is_some(v.big_vec) && (
            (vector::length(v.inline_vec) + big_vector::length<T>(option::borrow(v.big_vec))) > MAX_U64
        );
        ensures length(v) == length(old(v)) - 1;
    }

    spec swap {
        // TODO: temporarily mocked up.
        pragma verify = false;
    }

    spec append {
        pragma verify = false;
    }

    spec remove {
        pragma verify = false;
    }
}
