//# publish
module 0x815::m {

    enum CommonFields has drop {
      Foo{x: u64, y: u8},
      Bar{x: u64, y: u8, z: u32}
      Baz{y: u8}
    }

    fun update_common_field(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        common.x = 15;
        common.x
    }

    fun update_non_common_field(): u32 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        common.z = 15;
        common.z
    }

    fun update_common_field_different_offset(): u8 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        common.y = 15;
        common.y
    }
}

//# run 0x815::m::update_common_field

//# run 0x815::m::update_non_common_field

//# run 0x815::m::update_common_field_different_offset
