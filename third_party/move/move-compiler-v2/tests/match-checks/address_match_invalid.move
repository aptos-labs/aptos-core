module 0xc0ffee::address_match_invalid {
    fun match_address(a: address): u64 {
        match (a) {
            @0x1 => 1,
            _ => 0,
        }
    }
}
