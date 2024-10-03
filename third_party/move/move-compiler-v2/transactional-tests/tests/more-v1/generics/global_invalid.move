//# publish --print-bytecode
module 0x42::m {
    fun invalid<T:key + drop>(addr: address) {
        assert!(exists<T>(addr), 0);
        let _ = borrow_global<T>(addr);
        move_from<T>(addr);
    }
}

//# publish --print-bytecode
module 0x41::n {
    struct R has key, drop { f: u64 }

    public inline fun my_borrow() {
        invalid<R>(@0x41)
    }

    public fun test_resource(s: &signer) acquires R {
        move_to<R>(s, R{f:1});
        my_borrow().f;
    }
}

//# run --signers 0x41
script {
    use 0x42::M;
    fun main(account: signer) {
        M::test_resource(&account)
    }
}
