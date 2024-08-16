module 0x815::m {

    enum CommonFields has drop {
    Foo{x: u64, y: u8},
    Bar{x: u64, y: u8, z: u32}
    }

    fun t9_common_field(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        common.x = 15;
        common.x
    }
}
