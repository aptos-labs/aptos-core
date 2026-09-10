module 0x42::recursive_result_weakening {
    use std::vector;

    native fun unknown_result_internal(x: u64): u64;

    spec unknown_result_internal {
        pragma opaque;
        aborts_if false;
    }

    fun clone_result(x: &u64): u64 {
        unknown_result_internal(*x)
    }

    spec clone_result {
        aborts_if false;
    }

    inline fun map_ref(v: &vector<u64>, f: |&u64| u64): vector<u64> {
        let result = vector[];
        let i = 0;
        while (i < vector::length(v)) {
            vector::push_back(&mut result, f(vector::borrow(v, i)));
            i = i + 1;
        } spec {
            invariant result == spec_map_ref(f, v, i);
        };
        result
    }

    spec fun spec_map_ref(
        f: |&u64| u64, v: vector<u64>, end: u64
    ): vector<u64> {
        if (end == 0) vec()
        else concat(
            spec_map_ref(f, v, end - 1),
            vec(result_of<f>(v[end - 1]))
        )
    }

    fun unresolved_recursive_result_is_weakened(v: &vector<u64>) {
        let _ = map_ref(v, |x| clone_result(x));
    }
}
