module 0x99::GlobalAccess {
    use std::signer;
    use std::vector;

    struct S has key {
        val: u64,
    }

    struct S1 has key, drop {
        val: u64,
    }

    fun move_from_S1(account: &signer): S1 {
        move_from<S1>(signer::address_of(account))
    }

    fun dummy_func(): vector<u8> {
        vector::empty<u8>()
    }

    // `r1.val` can be reused
    // perf_gain: 1 call to `address_of` + 1 `borrow_global` + 1 `borrow_field` + 1 `readref` eliminated (counted as 6 bytecodes in total)
    // new_cost:
    // - `r1.val` flushed and copied twice
    // - the result of `r2.val` needs to be adjust on stack (one st_loc and one move_loc, counted as 2 bytecodes)
    // - total 6 bytecodes added
    fun test_global_borrow(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `r1.val` can be reused
    // performance gain same as `test_global_borrow`
    fun test_global_borrow_v2(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        move_from<S1>(signer::address_of(account));
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `r1.val` can be reused
    // performance gain same as `test_global_borrow`
    fun test_global_borrow_v3(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        move_from_S1(account);
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `r1.val` can be reused
    // performance gain same as `test_global_borrow`
    fun test_global_borrow_v4(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        dummy_func();
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `signer::address_of(account)` and `exists<S>(signer::address_of(account))` can be reused
    // perf_gain: two calls to `address_of` + one call to `exists` eliminated (counted as 9 bytecodes in total)
    // new_cost:
    // - result of `address_of` flushed and copied three times (counted as 4 bytecodes in total)
    // - result of first `exists` copied twice (counted as 3 bytecode in total)
    // - `b2` needs to be adjusted on stack (one st_loc and one move_loc, counted as 2 bytecodes)
    fun test_existence_check(account: &signer): bool {
        let b1 = exists<S>(signer::address_of(account));
        borrow_global<S>(signer::address_of(account));
        let b2 = exists<S>(signer::address_of(account));
        b1 && b2
    }
}
