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

    #[module_lock]
    fun callback_not_ok(): bool acquires R {
        let r = &mut R[@0x42];
        // This callback would be ok with regular lock, but is not because of module lock
        callee::call_me(r, |x| do_something(x));
        r.count += 1;
        assert!(r.count == 2);
        true
    }

    fun do_something(r: &mut R) {
        r.count += 1
    }
}


//# run 0x42::caller::init --signers 0x42

//# run 0x42::caller::callback_not_ok --verbose
