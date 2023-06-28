
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
}

script {
    use 0x100::M3;

    fun main() {
        let a = @0x2A3B;
        let b = @0x2A3B;
        assert!(M3::eq_address(a, b), 0xf00);

        let a = @0x55AA1122334455;
        let b = @0x55AA1122334456;
        assert!(M3::ne_address(a, b), 0xf01);
        assert!(!M3::eq_address(a, b), 0xf01);

        let a = @0x42;
        let t1 = M3::use_address_val(a);
        assert!(t1 == @0x42, 0xf02);

        let b = @0x000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F;
        let t2 = M3::use_address_ref(&b);
        assert!(M3::eq_address(t2, @0x000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F), 0xf03);

        let c = @0xc0ffee;
        let t3 = M3::ret_address_ref(&c);
        assert!(M3::eq_address(*t3, @0xc0ffee), 0xf04);

        assert!(M3::ne_address(@0xabba, @0xc0ffee), 0xf05);
    }
}
