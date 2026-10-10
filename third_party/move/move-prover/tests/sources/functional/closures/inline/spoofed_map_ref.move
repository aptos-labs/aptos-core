// A same-named function must not receive `std::vector::map_ref`'s summary.
module 0x42::spoofed_map_ref {
    use std::vector;

    struct R has key { value: u64 }

    inline fun map_ref<T, U>(v: &vector<T>, f: |&T| U): vector<U> {
        R[@0x42].value = R[@0x42].value + 1;
        let result = vector::empty<U>();
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            vector::push_back(&mut result, f(vector::borrow(v, i)));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant result == spec_map_ref(f, v, i);
            invariant !spec_map_ref_aborts(f, v, i);
        };
        result
    }

    spec fun spec_map_ref<T, U>(
        f: |&T| U, v: vector<T>, end: u64
    ): vector<U> {
        if (end == 0) vec()
        else concat(
            spec_map_ref(f, v, end - 1),
            vec(result_of<f>(v[end - 1]))
        )
    }

    spec fun spec_map_ref_aborts<T, U>(
        f: |&T| U, v: vector<T>, end: u64
    ): bool {
        end > 0 && (
            spec_map_ref_aborts(f, v, end - 1)
                || aborts_of<f>(v[end - 1])
        )
    }

    fun nested(v: &vector<vector<u64>>): vector<vector<u64>> acquires R {
        map_ref(v, |inner| map_ref(inner, |x| *x + 1))
    }
}
