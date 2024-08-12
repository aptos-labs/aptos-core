module 0xc0ffee::m {
    
    public fun is_empty<Element>(v: &vector<Element>): bool {
        use std::vector::length;
        length(v) == 0
    }

    public fun swap_remove<Element>(v: &mut vector<Element>, i: u64): Element {
        use std::vector::{length, pop_back, swap};
        assert!(!is_empty(v), 0);
        let last_idx = length(v) - 1;
        swap(v, i, last_idx);
        pop_back(v)
    }

}
