module 0x42::global_invariant {
    struct R has key {
        x: u64,
    }

    fun f(): u64 { 0 }

    spec module {
        invariant [global] forall a: address where exists<R>(a): global<R>(a).x > 0;
    }
}
