address 0x42 {
module M {
    struct R has key, store {}
    struct T has key {r: R}
    public fun new(): R {
        R {}
    }

    public fun new_t(): T {
        T {r: R {}}
    }

    public inline fun inline_borrow(addr: address): &R {
        borrow_global<R>(addr)
    }

    public inline fun inline_borrow_mut(addr: address): &R {
        borrow_global_mut<R>(addr)
    }

    public inline fun inline_move_to(account: &signer, r: R) {
        move_to<R>(account, r)
    }

    public inline fun inline_move_from(addr: address): R {
        move_from<R>(addr)
    }

    public inline fun inline_pack(): R {
        R {}
    }

    public inline fun inline_unpack(r: R) {
        let R{} = r;
    }

    public inline fun inline_access(t: T): R {
        t.r
    }

}
module M2 {
    use 0x42::M;

    fun test_borrow() {
        M::inline_borrow(@0x42);
    }

    fun test_borrow_mut() {
        M::inline_borrow_mut(@0x42);
    }

    fun test_move_to(account: signer) {
        let r = M::new();
        M::inline_move_to(&account, r);
    }

    fun test_move_from(addr: address) {
        M::inline_move_from(addr);
    }

    fun test_inline_pack() {
        M::inline_pack();
    }

    fun test_inline_unpack() {
        let r = M::new();
        M::inline_unpack(r);
    }

    fun test_inline_access() {
        let t = M::new_t();
        M::inline_access(t);
    }

}
}
