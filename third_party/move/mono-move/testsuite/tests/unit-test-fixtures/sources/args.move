/// Signer arguments by value and reference, address arguments, and mixed argument lists.
module fixtures::args {
    use std::signer;

    #[test(account = @0xcafe)]
    fun signer_by_value(account: signer) {
        assert!(signer::address_of(&account) == @0xcafe, 0);
    }

    #[test(account = @0xcafe)]
    fun signer_by_reference(account: &signer) {
        assert!(signer::address_of(account) == @0xcafe, 0);
    }

    #[test(addr = @0xcafe)]
    fun address_by_value(addr: address) {
        assert!(addr == @0xcafe, 0);
    }

    #[test(first = @0x1, second = @0x2, addr = @0x3)]
    fun mixed(first: signer, second: &signer, addr: address) {
        assert!(signer::address_of(&first) == @0x1, 0);
        assert!(signer::address_of(second) == @0x2, 1);
        assert!(addr == @0x3, 2);
    }
}
