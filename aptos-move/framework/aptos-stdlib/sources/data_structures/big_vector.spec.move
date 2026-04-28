spec aptos_std::big_vector {
    // -----------------
    // Data invariants
    // -----------------

    spec module {
        global initial_end_index: u64;
    }

    spec BigVector {
        invariant bucket_size != 0;
        invariant spec_table_len(buckets) == 0 ==> end_index == 0;
        invariant end_index == 0 ==> spec_table_len(buckets) == 0;
        invariant end_index <= spec_table_len(buckets) * bucket_size;

        // ensure all buckets except last has `bucket_size`
        invariant spec_table_len(buckets) == 0
            || (forall i in 0..spec_table_len(buckets)-1: len(table_with_length::spec_get(buckets, i)) == bucket_size);
        // ensure last bucket doesn't have more than `bucket_size` elements
        invariant spec_table_len(buckets) == 0
            || len(table_with_length::spec_get(buckets, spec_table_len(buckets) -1 )) <= bucket_size;
        // ensure each table entry exists due to a bad spec in `Table::spec_get`
        invariant forall i in 0..spec_table_len(buckets): spec_table_contains(buckets, i);
        // ensure no table key exists outside [0, spec_table_len).
        // explicit trigger avoids TypeDomain ($IsValid'u64') cascade: fires only on spec_table_contains ground terms.
        invariant forall i: u64 {spec_table_contains(buckets, i)}: spec_table_contains(buckets, i) ==> i < spec_table_len(buckets);
        // ensure bucket lengths add up to `end_index`
        // note: this together with the non-empty last bucket invariant and the full non-last
        // buckets invariant implies spec_table_len == ceil(end_index / bucket_size), so the
        // division-based formulation of that invariant is not needed here.
        invariant (spec_table_len(buckets) == 0 && end_index == 0)
            || (spec_table_len(buckets) != 0 && ((spec_table_len(buckets) - 1) * bucket_size) + (len(table_with_length::spec_get(buckets, spec_table_len(buckets) - 1))) == end_index);
        // ensures that no out-of-bound buckets exist
        invariant forall i: u64 {spec_table_contains(buckets, i)} where i >= spec_table_len(buckets):  {
            !spec_table_contains(buckets, i)
        };
        // ensures that the last bucket is non-empty
        invariant spec_table_len(buckets) == 0
            || (len(table_with_length::spec_get(buckets, spec_table_len(buckets) - 1)) > 0);
    }


    // -----------------------
    // Function specifications
    // -----------------------

    spec empty<T: store>(bucket_size: u64): BigVector<T> {
        aborts_if bucket_size == 0;
        ensures result.length() == 0;
        ensures result.bucket_size == bucket_size;
    }

    spec singleton<T: store>(element: T, bucket_size: u64): BigVector<T> {
        aborts_if bucket_size == 0;
        ensures result.length() == 1;
        ensures result.bucket_size == bucket_size;
    }

    spec destroy_empty<T>(self: BigVector<T>) {
        aborts_if !self.is_empty();
    }

    spec borrow<T>(self: &BigVector<T>, i: u64): &T {
        aborts_if i >= self.length();
        ensures result == spec_at(self, i);
    }

    spec borrow_mut<T>(self: &mut BigVector<T>, i: u64): &mut T {
        aborts_if i >= self.length();
        ensures result == spec_at(self, i);
    }

    spec push_back<T: store>(self: &mut BigVector<T>, val: T) {
        pragma opaque;
        let num_buckets = spec_table_len(self.buckets);
        include PushbackAbortsIf<T>;
        ensures self.length() == old(self).length() + 1;
        ensures self.end_index == old(self.end_index) + 1;
        ensures spec_at(self, self.end_index-1) == val;
        ensures forall i in 0..self.end_index-1: spec_at(self, i) == spec_at(old(self), i);
        ensures self.bucket_size == old(self).bucket_size;
    }

    spec schema PushbackAbortsIf<T> {
        self: BigVector<T>;
        let num_buckets = spec_table_len(self.buckets);
        aborts_if num_buckets * self.bucket_size > MAX_U64;
        aborts_if self.end_index + 1 > MAX_U64;
    }

    spec pop_back<T>(self: &mut BigVector<T>): T {
        pragma opaque;
        aborts_if self.is_empty();
        ensures self.length() == old(self).length() - 1;
        ensures result == old(spec_at(self, self.end_index-1));
        ensures forall i in 0..self.end_index: spec_at(self, i) == spec_at(old(self), i);
        // Bucket-level postcondition: all remaining bucket positions are unchanged.
        // Nested forall so b is in scope for the inner range expression.
        // The inner trigger spec_get(self.buckets, b) fires when borrow_mut is called in swap_remove,
        // avoiding the $InRange trigger issue with the spec_at bounded forall above.
        ensures forall b in 0..spec_table_len(self.buckets):
            forall p in 0..len(table_with_length::spec_get(self.buckets, b)):
                table_with_length::spec_get(self.buckets, b)[p] ==
                table_with_length::spec_get(old(self).buckets, b)[p];
        ensures self.bucket_size == old(self).bucket_size;
    }

    spec swap_remove<T>(self: &mut BigVector<T>, i: u64): T {
        pragma opaque;
        aborts_if i >= self.length();
        ensures self.length() == old(self).length() - 1;
        ensures result == spec_at(old(self), i);
    }

    spec swap<T>(self: &mut BigVector<T>, i: u64, j: u64) {
        pragma opaque;
        aborts_if i >= self.length() || j >= self.length();
        ensures self.length() == old(self).length();
        ensures spec_at(self, i) == spec_at(old(self), j);
        ensures spec_at(self, j) == spec_at(old(self), i);
        ensures forall idx in 0..self.length()
            where idx != i && idx != j:
            spec_at(self, idx) == spec_at(old(self), idx);
    }

    spec append<T: store>(self: &mut BigVector<T>, other: BigVector<T>) {
        ensures self.length() == old(self.length()) + other.length();
    }

    spec remove<T>(self: &mut BigVector<T>, i: u64): T {
        pragma opaque;
        aborts_if i >= self.length();
        ensures self.length() == old(self.length()) - 1;
        ensures self.bucket_size == old(self.bucket_size);
        ensures result == spec_at(old(self), i);
    }

    spec index_of<T>(self: &BigVector<T>, val: &T): (bool, u64) {
        pragma opaque;
        aborts_if false;
        ensures result_1 ==> result_2 < self.length();
        ensures result_1 ==> spec_at(self, result_2) == val;
        // Bucket-level formulation avoids the non-linear i/bucket_size arithmetic of spec_at.
        // Equivalent to forall i in 0..length(): spec_at(self, i) != val given BigVector invariants.
        ensures !result_1 ==> (forall j in 0..spec_table_len(self.buckets):
            forall k in 0..len(table_with_length::spec_get(self.buckets, j)):
                table_with_length::spec_get(self.buckets, j)[k] != val);
    }

    spec contains<T>(self: &BigVector<T>, val: &T): bool {
        aborts_if false;
    }

    // ---------------------
    // Spec helper functions
    // ---------------------

    spec fun spec_table_len<K, V>(t: TableWithLength<K, V>): u64 {
        table_with_length::spec_len(t)
    }

    spec fun spec_table_contains<K, V>(t: TableWithLength<K, V>, k: K): bool {
        table_with_length::spec_contains(t, k)
    }

    spec fun spec_at<T>(v: BigVector<T>, i: u64): T {
        let bucket = i / v.bucket_size;
        let idx = i % v.bucket_size;
        let v = table_with_length::spec_get(v.buckets, bucket);
        v[idx]
    }
}
