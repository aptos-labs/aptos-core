
// Check generics with primitive types.
//
// Also check that instantiations of the same generic from two different
// modules don't have name collision.
module 0x100::MX {
    public fun generic_id<T>(v: T): T {
        v
    }
}

module 0x200::MX {
    use 0x100::MX::generic_id;

    public fun square32(n: u32): u32 {
        generic_id<u32>(n) * generic_id<u32>(n)
    }

    public fun square8(n: u8): u8 {
        generic_id<u8>(n) * generic_id<u8>(n)
    }
}

script {
    fun main() {
        let x2 = 0x200::MX::square8(8);
        assert!(x2 == 64, 0xf00);

        let y2 = 0x200::MX::square32(32);
        assert!(y2 == 1024, 0xf01);

        // Second instantiation of generic_id<u8>.
        let z = 0x100::MX::generic_id<u8>(33);
        assert!(z == 33, 0xf02);
    }
}
