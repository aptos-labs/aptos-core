// signers 0xcafe,0xf00d,0xc0ffee,0xb00

// A phony `signer` module until we build `move-stdlib`.
module 0x500::signer {
    native public fun borrow_address(acct: &signer): &address;

    // Copies the address of the signer
    public fun address_of(s: &signer): address {
        *borrow_address(s)
    }
}

module 0x100::M5 {
    use 0x500::signer;

    public fun signer_by_val(a: signer) {
        assert!(signer::address_of(&a) == @0xcafe, 0xf00);
        assert!(*signer::borrow_address(&a) == @0xcafe, 0xf01);
    }

    public fun signer_by_ref(a: &signer) {
        assert!(signer::address_of(a) == @0xf00d, 0xf02);
    }

    public fun signers_by_ref(a: &signer, b: &signer) {
        assert!(*signer::borrow_address(a) == @0xc0ffee, 0xf03);
        assert!(*signer::borrow_address(b) == @0xb00, 0xf04);
    }
}

script {
    use 0x100::M5;

    fun main(s1: signer, s2: signer, s3: signer, s4: signer) {
        M5::signer_by_val(s1);
        M5::signer_by_ref(&s2);
        M5::signers_by_ref(&s3, &s4);
    }
}
