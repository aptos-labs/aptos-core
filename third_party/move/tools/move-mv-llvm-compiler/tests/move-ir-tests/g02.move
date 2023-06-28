

module 0x100::M6 {
    struct Foo<T> has copy, drop { x: T }

    struct Bar<T1, T2> has copy, drop {
        x: T1,
        y: T2,
    }

    struct Baz<T1, T2> has copy, drop {
        x: T1,
        y: Foo<T2>,
    }

    fun boo(): bool {
        let t1 = Foo<bool> { x: true };
        let Foo<bool> { x } = t1;
        x
    }

    fun goo(): (u8, u64) {
        let t1 = Bar<u8, u64> { x: 123, y: 456 };
        let Bar<u8, u64> { x, y } = t1;
        (x, y)
    }

    fun zoo(): (u8, u64) {
        let ffs = Foo<u64> { x: 1992 };
        let t1 = Baz<u8, u64> { x: 123, y: ffs };
        let Baz<u8, u64> { x, y } = t1;
        (x, y.x)
    }

    fun snd_rcv(a: Foo<u16>): Foo<u16> {
        a
    }

    fun rcv_and_idx(a: Baz<address, u32>): u32 {
        a.y.x
    }
}
