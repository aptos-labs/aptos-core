module 0x99::GlobalAccess {
    use std::signer;
    use std::vector;

    struct S has key, drop {
        val: u64,
    }

    struct S1 has key, drop {
        val: u64,
    }

    fun move_to_S(account: &signer) {
        move_to<S>(account, S { val: 42 } )
    }

    fun move_from_S(account: &signer) {
        move_from<S>(signer::address_of(account));
    }

    fun mutable_borrow_S(account: &signer) {
        *borrow_global_mut<S>(signer::address_of(account)) = S { val: 100  };
    }

    fun dummy_func(): vector<u8> {
        vector::empty<u8>()
    }

    // `borrow_global<S>(addr).val` cannot be reused
    // because `move_to_S` may modify the global storage
    fun test_global_borrow_v1(account: &signer): u64 {
        let addr = signer::address_of(account);
        let r1 = borrow_global<S>(addr);
        move_to_S(account);
        let r2 = borrow_global<S>(addr);
        r1.val + r2.val
    }

    // `exists<S>(addr)` cannot be reused
    // because `move_to_S` may modify the global storage
    fun test_existence_check_v1(account: &signer): bool {
        let addr = signer::address_of(account);
        let b1 = exists<S>(addr);
        move_to_S(account);
        let b2 = exists<S>(addr);
        b1 && b2
    }

    // `exists<S>(addr)` cannot be reused
    // because `move_from_S` may modify the global storage
    fun test_existence_check_v2(account: &signer): bool {
        let addr = signer::address_of(account);
        let b1 = exists<S>(addr);
        move_from_S(account);
        let b2 = exists<S>(addr);
        b1 && b2
    }

    // `exists<S>(addr)` cannot be reused
    // because `move_to_S` may modify the global storage
    fun test_existence_check_v3(account: &signer): bool {
        let addr = signer::address_of(account);
        let b1 = exists<S>(addr);
        mutable_borrow_S(account);
        let b2 = exists<S>(addr);
        b1 && b2
    }
}
