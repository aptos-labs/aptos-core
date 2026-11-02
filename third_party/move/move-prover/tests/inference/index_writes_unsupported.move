// A write through a reference a callee returned into a vector element, or into
// one of several places, depends on a choice made inside the callee. WP reports
// it instead of inferring a post-state without the write.
module 0x42::index_writes_unsupported {
    struct S has drop {
        a: u64,
        b: u64,
    }

    fun elem(v: &mut vector<u64>, i: u64): &mut u64 {
        &mut v[i]
    }

    fun set_elem(v: &mut vector<u64>, i: u64, x: u64) {
        *elem(v, i) = x;
    }

    fun pick(s: &mut S, first: bool): &mut u64 {
        if (first) &mut s.a else &mut s.b
    }

    fun set_picked(s: &mut S, first: bool, x: u64) {
        *pick(s, first) = x;
    }
}
