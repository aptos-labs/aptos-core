//# publish
// Should succeed.
module 0x42::test_case {
    use std::vector;

    /// The index into the vector is out of bounds
    const EINVALID_RANGE: u64 = 0x20001;

    /// Apply the function to a reference of each element in the vector.
    public inline fun for_each_ref<Element>(v: &vector<Element>, f: |&Element|) {
        let i = 0;
        while (i < vector::length(v)) {
            f(vector::borrow(v, i));
            i = i + 1
        }
    }

    /// Map the function over the references of the elements of the vector, producing a new vector without modifying the
    /// original map.
    public inline fun map_ref<Element, NewElement>(
        v: &vector<Element>,
        f: |&Element|NewElement
    ): vector<NewElement> {
        let result = vector<NewElement>[];
        for_each_ref(v, |elem| vector::push_back(&mut result, f(elem)));
        result
    }

    /// Reverses the order of the elements [left, right) in the vector `v` in place.
    public fun reverse_slice<Element>(v: &mut vector<Element>, left: u64, right: u64) {
        assert!(left <= right, EINVALID_RANGE);
        if (left == right) return;
        right = right - 1;
        while (left < right) {
            vector::swap(v, left, right);
            left = left + 1;
            right = right - 1;
        }
    }
    /// Same as above but on a sub-slice of an array [left, right) with left <= rot <= right
    /// returns the
    public fun rotate_slice<Element>(
        v: &mut vector<Element>,
        left: u64,
        rot: u64,
        right: u64
    ): u64 {
        reverse_slice(v, left, rot);
        reverse_slice(v, rot, right);
        reverse_slice(v, left, right);
        left + (right - rot)
    }

    /// For in-place stable partition we need recursion so we cannot use inline functions
    /// and thus we cannot use lambdas. Luckily it so happens that we can precompute the predicate
    /// in a secondary array. Note how the algorithm belows only start shuffling items after the
    /// predicate is checked.
    public fun stable_partition_internal<Element>(
        v: &mut vector<Element>,
        pred: &vector<bool>,
        left: u64,
        right: u64
    ): u64 {
        if (left == right) {
            left
        } else if (left + 1 == right) {
            if (*vector::borrow(pred, left)) right else left
        } else {
            let mid = left + ((right - left) >> 1);
            let p1 = stable_partition_internal(v, pred, left, mid);
            let p2 = stable_partition_internal(v, pred, mid, right);
            rotate_slice(v, p1, mid, p2)
        }
    }

    /// Partition the array based on a predicate p, this routine is stable and thus
    /// preserves the relative order of the elements in the two partitions.
    public inline fun stable_partition<Element>(
        v: &mut vector<Element>,
        p: |&Element|bool
    ): u64 {
        let pred = map_ref(v, |e| p(e));
        let len = vector::length(v);
        stable_partition_internal(v, &pred,0, len)
    }

    fun test_stable_partition() {
        let v = vector[1, 2, 3, 4, 5];
        let t = stable_partition(&mut v, |n| *n % 2 == 0);
        assert!(t == 2, 0);
        assert!(&v == &vector[2, 4, 1, 3, 5], 1);
    }
}
