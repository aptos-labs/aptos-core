
module 0x100::M10 {
    struct Foo<T> has copy, drop { x: T }

    struct Bar<T1, T2> has copy, drop {
        x: T1,
        y: T2,
    }

    public fun f1_pack_unpack_1type(in: bool): bool {
        let t1 = Foo<bool> { x: in };
        let Foo<bool> { x } = t1;
        x
    }

    public fun f1_pack_unpack_2types(in1: u8, in2: u64): (u8, u64) {
        let t1 = Bar<u8, u64> { x: in1, y: in2 };
        let Bar<u8, u64> { x, y } = t1;
        (x + 1, y + 1)
    }

    public fun new_foo_u16(val: u16): Foo<u16> {
        Foo<u16> { x: val }
    }

    public fun get_from_foo_u16(s: Foo<u16>): u16 {
        s.x
    }

    public fun get_from_foo_u16_byref(s: &Foo<u16>): u16 {
        s.x
    }
}

script {
    use 0x100::M10;

    fun main() {
        let t1 = M10::f1_pack_unpack_1type(true);
        assert!(t1 == true, 0xf00);
        let t2 = M10::f1_pack_unpack_1type(false);
        assert!(t2 == false, 0xf01);

        let (t3, t4) = M10::f1_pack_unpack_2types(123, 456);
        assert!(t3 == 124, 0xf02);
        assert!(t4 == 457, 0xf03);

        let t5 = M10::new_foo_u16(65000);
        assert!(M10::get_from_foo_u16(t5) == 65000, 0xf04);

        let t6 = M10::new_foo_u16(0xf00d);
        assert!(M10::get_from_foo_u16_byref(&t6) == 0xf00d, 0xf05);
    }
}
