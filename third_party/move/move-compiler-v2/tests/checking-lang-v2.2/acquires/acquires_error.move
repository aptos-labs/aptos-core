module 0x42::acquires_inferred {

    struct R has key, store { f: u64 }
    struct S has key, store { f: u64 }

    fun publish(s: &signer) {
        move_to(s, R{f: 1});
    }

    fun check(a: address): bool {
        exists<R>(a)
    }

    fun read(a: address): u64 acquires S {
        let r = borrow_global<R>(a);
        r.f
    }

    fun write(a: address, x: u64): u64 {
        let r = borrow_global_mut<R>(a);
        r.f = x;
        9
    }
}
