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

    // `borrow_global<S>(addr).val` can be reused
    // perf_gain: 1 call to `address_of` + 1 global access + 1 readref eliminated
    // new_cost:
    // - `u64` flushed and copied twice
    // - the result of `r2.val` needs to be adjust on stack (one st_loc and one move_loc)
    fun test_global_borrow(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `borrow_global<S>(addr)` can be reused
    // perf_gain: 1 call to `address_of` + 1 global access + 1 readref eliminated
    // new_cost:
    // - `u64` flushed and copied twice
    // - the result of `r2.val` needs to be adjust on stack (one st_loc and one move_loc)
    fun test_global_borrow_v2(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        move_from<S1>(signer::address_of(account));
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `borrow_global<S>(addr)` can be reused
    // perf_gain: 1 call to `address_of` + 1 global access + 1 readref eliminated
    // new_cost:
    // - `u64` flushed and copied twice
    // - the result of `r2.val` needs to be adjust on stack (one st_loc and one move_loc)
    fun test_global_borrow_v3(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        move_from_S1(account);
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `borrow_global<S>(addr)` can be reused
    // perf_gain: 1 call to `address_of` + 1 global access + 1 readref eliminated
    // new_cost:
    // - `u64` flushed and copied twice
    // - the result of `r2.val` needs to be adjust on stack (one st_loc and one move_loc)
    fun test_global_borrow_v4(account: &signer): u64 {
        let r1 = borrow_global<S>(signer::address_of(account));
        dummy_func();
        let r2 = borrow_global<S>(signer::address_of(account));
        r1.val + r2.val
    }

    // `exists<S>(addr)` can be reused
    // perf_gain: 1 call to `address_of` and 1 global existence check eliminated
    // new_cost:
    // - `bool` flushed and copied twice
    fun test_existence_check(account: &signer): bool {
        let b1 = exists<S>(signer::address_of(account));
        borrow_global<S>(signer::address_of(account));
        let b2 = exists<S>(signer::address_of(account));
        b1 && b2
    }
}
