// Provides a framework for converting well-known types to and from bytes
module aptos_framework::byte_conversions {
    use aptos_framework::util;
    use std::bcs;

    public fun to_address(bytes: vector<u8>): address {
        util::from_bytes(bytes)
    }

    public fun from_address(addr: &address): vector<u8> {
        bcs::to_bytes(addr)
    }

    #[test]
    fun test_correct() {
        let addr = @0x01;
        let addr_vec = x"0000000000000000000000000000000000000000000000000000000000000001";
        let addr_out = to_address(addr_vec);
        let addr_vec_out = from_address(&addr_out);
        assert!(addr == addr_out, 0);
        assert!(addr_vec == addr_vec_out, 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x10001)]
    fun test_incorrect() {
        let bad_vec = b"01";
        to_address(bad_vec);
    }
}
