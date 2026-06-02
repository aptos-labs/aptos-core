// Mixed parametric rows: one passes, one expects an arbitrary abort,
// one expects a specific abort_code with location.
address 0x1 {
module M {
    const E_BAD: u64 = 5;

    #[test(addr = @0x1)]
    #[test(addr = @0x2), expected_failure]
    #[test(addr = @0xdead), expected_failure(abort_code = 5, location = 0x1::M)]
    fun mixed(addr: signer) {
        if (std::signer::address_of(&addr) == @0x2) abort 1;
        if (std::signer::address_of(&addr) == @0xdead) abort E_BAD;
    }
}
}
