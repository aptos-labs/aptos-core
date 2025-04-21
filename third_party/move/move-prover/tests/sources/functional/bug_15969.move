module 0x42::m {
    use std::vector;

    struct S {
        v: vector<u64>
    }


    fun empty(self: &S) : bool {
        vector::length(&self.v) == 0
    }


    spec fun spec_empty(self: S): bool {
        self.empty()
    }

    fun test_empty(s: &S): bool {
        s.empty()
    }

    spec test_empty {
        ensures result != spec_empty(s); // this does not verify
    }



}
