spec aptos_std::smart_vector {

    spec SmartVector {
        // `bucket_size` shouldn't be 0, if specified.
        invariant option::spec_is_none(bucket_size)
            || (option::spec_is_some(bucket_size) && option::spec_borrow(bucket_size) != 0);
        // vector length should be <= `inline_capacity`, if specified.
        invariant option::spec_is_none(inline_capacity)
            || (len(inline_vec) <= option::spec_borrow(inline_capacity));
        // both `inline_capacity` and `bucket_size` should either exist or shouldn't exist at all.
        invariant (option::spec_is_none(inline_capacity) && option::spec_is_none(bucket_size))
            || (option::spec_is_some(inline_capacity) && option::spec_is_some(bucket_size));
    }

    spec length {
        aborts_if option::spec_is_some(self.big_vec) && len(self.inline_vec) + option::spec_borrow(
            self.big_vec).length() > MAX_U64;
    }

    spec empty {
        aborts_if false;
    }

    spec empty_with_config {
        aborts_if bucket_size == 0;
    }

    spec destroy_empty {
        aborts_if !(spec_is_empty(self));
        aborts_if len(self.inline_vec) != 0
            || option::spec_is_some(self.big_vec);
    }

    spec borrow {
        aborts_if i >= spec_len(self);
        aborts_if option::spec_is_some(self.big_vec) && (
            (len(self.inline_vec) + option::spec_borrow(self.big_vec).length::<T>()) > MAX_U64
        );
    }

    spec push_back<T: store>(self: &mut SmartVector<T>, val: T) {
        // use aptos_std::big_vector;
        // use aptos_std::type_info;
        pragma verify = false; // TODO: set to false because of timeout
        // pragma aborts_if_is_partial;
        // let pre_length = length(v);
        // let pre_inline_len = len(v.inline_vec);
        // let pre_big_vec = option::spec_borrow(v.big_vec);
        // let post post_big_vec = option::spec_borrow(v.big_vec);
        // let size_val = type_info::spec_size_of_val(val);
        // include pre_length != pre_inline_len ==> big_vector::PushbackAbortsIf<T> {
        //     v: pre_big_vec
        // };
        // aborts_if pre_length == pre_inline_len && option::is_none(v.inline_capacity) && (pre_length + 1) > MAX_U64;
        // aborts_if pre_length == pre_inline_len && option::is_none(v.inline_capacity) && size_val * (pre_length + 1) > MAX_U64;
        // aborts_if pre_length == pre_inline_len && option::is_none(v.inline_capacity) && size_val * (pre_length + 1) >= 150 && size_val + type_info::spec_size_of_val(v.inline_vec) > MAX_U64;
        // aborts_if option::is_some(v.big_vec) && len(v.inline_vec) + big_vector::length(option::spec_borrow(v.big_vec)) > MAX_U64;
        // aborts_if pre_length == pre_inline_len && option::is_none(v.inline_capacity) && size_val * (pre_length + 1) >= 150 && option::is_some(v.big_vec);
        // aborts_if pre_length == pre_inline_len && option::is_some(v.inline_capacity) && pre_length >= option::spec_borrow(v.inline_capacity) && option::is_some(v.big_vec);
        // ensures pre_length != pre_inline_len ==> option::is_some(v.big_vec);
        // ensures pre_length != pre_inline_len ==> big_vector::spec_at(post_big_vec, post_big_vec.end_index-1) == val;
    }

    spec pop_back {
        use aptos_std::table_with_length;

        pragma verify_duration_estimate = 120; // TODO: set because of timeout (property proved)

        aborts_if  option::spec_is_some(self.big_vec)
            &&
            (table_with_length::spec_len(option::spec_borrow(self.big_vec).buckets) == 0);
        aborts_if spec_is_empty(self);
        aborts_if option::spec_is_some(self.big_vec) && (
            (len(self.inline_vec) + option::spec_borrow(self.big_vec).length::<T>()) > MAX_U64
        );

        ensures spec_len(self) == old(spec_len(self)) - 1;
    }

    spec swap_remove {
        pragma verify = false; // TODO: set because of timeout
        aborts_if i >= spec_len(self);
        aborts_if option::spec_is_some(self.big_vec) && (
            (len(self.inline_vec) + option::spec_borrow(self.big_vec).length::<T>()) > MAX_U64
        );
        ensures spec_len(self) == old(spec_len(self)) - 1;
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

    spec singleton {
        pragma verify = false;
    }

    spec fun spec_len<T>(self: &SmartVector<T>): u64 {
        self.inline_vec.length() + if (option::spec_is_none(self.big_vec)) {
            0
        } else {
            option::spec_borrow(self.big_vec).length()
        }
    }

    spec fun spec_is_empty<T>(self: &SmartVector<T>): bool {
        spec_len(self) == 0
    }
}
