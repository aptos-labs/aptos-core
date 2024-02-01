module 0x1::signer {
    public fun address_of(arg0: &signer) : address {
        *borrow_address(arg0)
    }
    
    native public fun borrow_address(arg0: &signer) : &address;
    // decompiled from Move bytecode v6
}
