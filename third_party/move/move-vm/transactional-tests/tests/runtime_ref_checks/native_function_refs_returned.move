//# publish
module 0xc0ffee::m {
    use std::signer::{borrow_address, address_of};

    public fun test(s: &signer) {
        let addr = borrow_address(s);
        assert!(address_of(s) == *addr, 1);
    }
}

//# run 0xc0ffee::m::test --signers 0xc0ffee --verbose
