address 0x1 {
module parametric_case_identity {
    #[test(addr = @0x1)]
    #[test(addr = @0x2)]
    fun executes_original_function(addr: signer) {
        let value = std::signer::address_of(&addr);
        assert!(value == @0x1 || value == @0x2, 0);
    }
}
}
