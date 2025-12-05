//# publish
module 0xcafe::signer_native_returns {
    use 0x1::signer;

    public entry fun borrow_address_roundtrip(account: signer): address {
        let addr_ref = signer::borrow_address(&account);
        *addr_ref
    }
}

//# run 0xcafe::signer_native_returns::borrow_address_roundtrip --signers 0xcafe
