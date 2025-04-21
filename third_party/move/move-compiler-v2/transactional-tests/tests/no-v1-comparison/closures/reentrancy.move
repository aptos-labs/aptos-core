//# publish
module 0x42::callee {
    public fun call_me<T>(x: &mut T, action: |&mut T|) {
        action(x)
    }
}

//# publish
module 0x42::caller {
    use 0x42::callee;
    struct R has key, copy, drop {
        count: u64
    }

    fun init(s: &signer) {
        move_to(s, R{count: 0});
    }

    fun callback_ok(): bool acquires R {
        let r = &mut R[@0x42];
        // This callback is OK, because `R` is not acquired
        callee::call_me(r, |x| do_something(x));
        r.count += 1;
        assert!(r.count == 2);
        true
    }

    fun callback_fails(): bool acquires R {
        let r = &mut R[@0x42];
        // This callback will lead to reentrancy runtime error
        callee::call_me(r, |_| R[@0x42].count += 1);
        r.count += 1;
        assert!(r.count == 2);
        false
    }

    fun do_something(r: &mut R) {
        r.count += 1
    }
}


//# run 0x42::caller::init --signers 0x42

//# run 0x42::caller::callback_ok

//# run 0x42::caller::callback_fails --verbose
