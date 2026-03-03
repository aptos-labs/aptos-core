// Do not include stdlib for this test:
// no-stdlib

module 0x1::signer { // must be this module

    native fun borrow_address(self: &signer): &address;

    fun address_of(self: &signer): address {
        *borrow_address(self)
    }

    fun test_receiver_calls(s: signer) {
        s.address_of();
        s.borrow_address();
    }
}
