module 0x42::m {
    public fun foo(vec: &mut vector<bool>) {
        spec {
            assert forall k in 0..len(vec): vec[k] == old(vec)[k];
        };
    }
}
