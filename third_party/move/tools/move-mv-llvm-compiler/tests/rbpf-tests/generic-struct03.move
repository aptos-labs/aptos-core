

module 0x100::M10 {
    struct Foo<T> has copy, drop { x: T }

    // Generic struct with generic struct member.
    struct Bar<T1, T2> has copy, drop {
        a: T1,
        b: Foo<T2>,
    }

    public fun f1_pack_unpack_2types(in1: u8, in2: u64): (u8, u64) {
        let t0 = Foo<u64> { x: in2 };
        let t1 = Bar<u8, u64> { a: in1, b: t0 };
        let Bar<u8, u64> { a, b } = t1;
        (a + 1, b.x + 1)
    }

    public fun new_bar(in1: u8, in2: u64): Bar<u8, u64> {
        let t0 = Foo<u64> { x: in2 };
        let t1 = Bar<u8, u64> { a: in1, b: t0 };
        t1
    }

    public fun get_inner(s: Bar<u8, u64>): Foo<u64> {
        s.b
    }

    public fun get_from_foo(s: Foo<u64>): u64 {
        s.x
    }
}

script {
    use 0x100::M10;

    fun main() {
        let (t1, t2) = M10::f1_pack_unpack_2types(123, 456);
        assert!(t1 == 124, 0xf00);
        assert!(t2 == 457, 0xf01);

        let t3 = M10::new_bar(255, 65539);
        let t4 = M10::get_inner(t3);
        let t5 = M10::get_from_foo(t4);
        assert!(t5 == 65539, 0xf03);
    }
}
