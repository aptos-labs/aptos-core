module 0x42::globals {

    struct R has store { f: u64 }

    fun publish(s: &signer) {
        move_to(s, R{f: 1});
    }

    fun check(a: address): bool {
        exists<R>(a)
    }

    fun read(a: address): u64 {
        let r = borrow_global<R>(a);
        r.f
    }

    fun write(a: address, x: u64): u64 {
        let r = borrow_global_mut<R>(a);
        r.f = 2;
        9
    }
}
