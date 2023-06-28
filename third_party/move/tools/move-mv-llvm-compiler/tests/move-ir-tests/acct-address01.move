
module 0x100::M3 {
    public fun use_address_val(a: address): address  {
        let a2 = move a;
        a2
    }

    public fun use_address_ref(a: &address): address  {
        let a2 = *a;
        a2
    }

    public fun ret_address_ref(a: &address): &address  {
        a
    }

    public fun eq_address(a: address, b: address): bool {
        a == b
    }

    public fun ne_address(a: address, b: address): bool {
        a != b
    }

    public fun fixed_address(): address {
        @0x000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F
    }
}
