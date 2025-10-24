//# publish
module 0x42::vector {
    fun create(): vector<u64> {
        vector[1, 2, 3]
    }

    public fun remove<Element>(v: &mut vector<Element>, i: u64): Element {
        use std::vector;
        let len = vector::length(v);
        if (i >= len) abort 1;

        len = len - 1;
        while (i < len) vector::swap(v, i, { i = i + 1; i });
        vector::pop_back(v)
    }
}

//# run 0x42::vector::test_remove
