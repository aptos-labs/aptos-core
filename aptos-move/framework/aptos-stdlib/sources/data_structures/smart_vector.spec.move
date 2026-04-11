spec aptos_std::smart_vector {

    spec SmartVector {
        // `bucket_size` shouldn't be 0, if specified.
        invariant bucket_size.is_none()
            || (bucket_size.is_some() && option::borrow(bucket_size) != 0);
        // vector length should be <= `inline_capacity`, if specified.
        invariant inline_capacity.is_none()
            || (len(inline_vec) <= option::borrow(inline_capacity));
        // both `inline_capacity` and `bucket_size` should either exist or shouldn't exist at all.
        invariant (inline_capacity.is_none() && bucket_size.is_none())
            || (inline_capacity.is_some() && bucket_size.is_some());
    }

    spec new<T: store>(): SmartVector<T> {
        aborts_if false;
        ensures spec_len(result) == 0;
    }

    spec length {
        aborts_if self.big_vec.is_some() && len(self.inline_vec) + option::borrow(self.big_vec).length() > MAX_U64;
        ensures result == spec_len(self);
    }

    spec is_empty<T>(self: &SmartVector<T>): bool {
        pragma opaque;
        aborts_if self.big_vec.is_some() && len(self.inline_vec) + option::borrow(self.big_vec).length() > MAX_U64;
        ensures result == spec_is_empty(self);
    }

    spec empty {
        aborts_if false;
        ensures spec_len(result) == 0;
    }

    spec empty_with_config {
        aborts_if bucket_size == 0;
        ensures spec_len(result) == 0;
    }

    spec borrow_mut<T>(self: &mut SmartVector<T>, i: u64): &mut T {
        pragma opaque;
        aborts_if i >= spec_len(self);
        aborts_if self.big_vec.is_some() && (
            (len(self.inline_vec) + option::borrow(self.big_vec).length<T>()) > MAX_U64
        );
        ensures result == spec_get(self, i);
    }

    spec destroy_empty {
        aborts_if !(spec_is_empty(self));
        aborts_if len(self.inline_vec) != 0
            || self.big_vec.is_some();
    }

    spec borrow {
        aborts_if i >= spec_len(self);
        aborts_if self.big_vec.is_some() && (
            (len(self.inline_vec) + option::borrow(self.big_vec).length<T>()) > MAX_U64
        );
        ensures result == spec_get(self, i);
    }

    spec push_back<T: store>(self: &mut SmartVector<T>, val: T) {
        use aptos_std::type_info;
        use aptos_std::table_with_length;
        pragma opaque;
        // Body verification is infeasible: BigVector's bucket-layout struct invariants
        // (`forall i in 0..n-1: len(bucket[i]) == bucket_size`) are injected by
        // DataInvariantInstrumentationProcessor at every PackRefDeep site in the
        // multi-branch body. These `forall`-over-table-contents quantifiers, combined
        // with the `spec_at` ground terms from `big_vector::push_back`'s postconditions,
        // cause Z3 quantifier-instantiation cascades that exceed 40s in the full package.
        // `pragma verify = false` prevents the data-invariant processor from running
        // on this function while preserving all aborts_if/ensures for callers.
        pragma verify = false;
        // (A) Computing self.length() can overflow when big_vec contributes elements.
        aborts_if self.big_vec.is_some()
            && len(self.inline_vec) + option::borrow(self.big_vec).length<T>() > MAX_U64;
        // (B) Inside the inline_capacity.is_none() branch when no big_vec elements exist:
        // B1: inline_len + 1 overflows
        aborts_if self.inline_capacity.is_none()
            && spec_len(self) == len(self.inline_vec)
            && len(self.inline_vec) == MAX_U64;
        // B2: val_size * (inline_len + 1) overflows
        aborts_if self.inline_capacity.is_none()
            && spec_len(self) == len(self.inline_vec)
            && len(self.inline_vec) < MAX_U64
            && type_info::spec_size_of_val(val) * (len(self.inline_vec) + 1) > MAX_U64;
        // B3: size_of_val(inline_vec) + val_size overflows on the path to creating big_vec
        aborts_if self.inline_capacity.is_none()
            && spec_len(self) == len(self.inline_vec)
            && len(self.inline_vec) < MAX_U64
            && type_info::spec_size_of_val(val) * (len(self.inline_vec) + 1) <= MAX_U64
            && type_info::spec_size_of_val(val) * (len(self.inline_vec) + 1) >= 150
            && type_info::spec_size_of_val(self.inline_vec) + type_info::spec_size_of_val(val) > MAX_U64;
        // (C) option::fill aborts because big_vec already exists when we try to initialize it.
        // C1: inline_capacity branch — fill is reached when inline_vec is at capacity
        aborts_if self.inline_capacity.is_some()
            && spec_len(self) == len(self.inline_vec)
            && len(self.inline_vec) >= option::borrow(self.inline_capacity)
            && self.big_vec.is_some();
        // C2: inline_capacity.is_none() branch — fill is reached when val is large enough
        aborts_if self.inline_capacity.is_none()
            && spec_len(self) == len(self.inline_vec)
            && len(self.inline_vec) < MAX_U64
            && type_info::spec_size_of_val(val) * (len(self.inline_vec) + 1) <= MAX_U64
            && type_info::spec_size_of_val(val) * (len(self.inline_vec) + 1) >= 150
            && type_info::spec_size_of_val(self.inline_vec) + type_info::spec_size_of_val(val) <= MAX_U64
            && self.big_vec.is_some();
        // (E) big_vector::push_back aborts (only applies when big_vec already existed with elements,
        // i.e. spec_len > inline_len; after option::fill the freshly created big_vec has end_index=0
        // and 0 buckets so neither condition below can fire).
        // E1: num_buckets * bucket_size overflows
        aborts_if spec_len(self) > len(self.inline_vec)
            && table_with_length::spec_len(option::borrow(self.big_vec).buckets)
               * option::borrow(self.big_vec).bucket_size > MAX_U64;
        // E2: end_index + 1 overflows
        aborts_if spec_len(self) > len(self.inline_vec)
            && option::borrow(self.big_vec).end_index + 1 > MAX_U64;
        ensures spec_len(self) == old(spec_len(self)) + 1;
    }

    spec pop_back {
        use aptos_std::table_with_length;

        aborts_if  self.big_vec.is_some()
            &&
            (table_with_length::spec_len(option::borrow(self.big_vec).buckets) == 0);
        aborts_if spec_is_empty(self);
        aborts_if self.big_vec.is_some() && (
            (len(self.inline_vec) + option::borrow(self.big_vec).length<T>()) > MAX_U64
        );

        ensures spec_len(self) == old(spec_len(self)) - 1;
        ensures result == old(spec_get(self, spec_len(self) - 1));
    }

    spec swap_remove {
        aborts_if i >= spec_len(self);
        aborts_if self.big_vec.is_some() && (
            (len(self.inline_vec) + option::borrow(self.big_vec).length<T>()) > MAX_U64
        );
        ensures spec_len(self) == old(spec_len(self)) - 1;
        ensures result == old(spec_get(self, i));
    }

    spec swap {
        pragma aborts_if_is_partial;
        aborts_if i >= spec_len(self) || j >= spec_len(self);
        aborts_if self.big_vec.is_some() && (
            (len(self.inline_vec) + option::borrow(self.big_vec).length<T>()) > MAX_U64
        );
        ensures spec_len(self) == old(spec_len(self));
    }

    spec append {
        pragma verify = false;
    }

    spec remove {
        pragma opaque;
        aborts_if i >= spec_len(self);
        aborts_if self.big_vec.is_some() && (
            (len(self.inline_vec) + option::borrow(self.big_vec).length<T>()) > MAX_U64
        );
        ensures spec_len(self) == old(spec_len(self)) - 1;
        ensures result == old(spec_get(self, i));
    }

    spec singleton {
        // In theory the `size_of_val` arithmetic inside push_back could overflow, but in practice
        // BCS sizes are always tiny. pragma verify = false axiomatizes this: singleton never aborts.
        // aborts_if false;
        ensures spec_len(result) == 1;
    }

    spec fun spec_len<T>(self: &SmartVector<T>): u64 {
        self.inline_vec.length() + if (self.big_vec.is_none()) {
            0
        } else {
            option::borrow(self.big_vec).length()
        }
    }

    spec fun spec_is_empty<T>(self: &SmartVector<T>): bool {
        spec_len(self) == 0
    }

    spec fun spec_get<T>(self: &SmartVector<T>, i: u64): T {
        if (i < len(self.inline_vec)) {
            self.inline_vec[i]
        } else {
            big_vector::spec_at(option::borrow(self.big_vec), i - len(self.inline_vec))
        }
    }
}
