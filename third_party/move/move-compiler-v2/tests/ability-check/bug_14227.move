module 0x815::m {

    enum CommonFields has drop {
    Foo{x: u64, y: u8},
    Bar{x: u64, y: u8, z: u32}
    }

    enum CommonFieldsVector has drop {
    Foo{x: vector<u8>},
    Bar{x: vector<u8>, y: vector<CommonFields>}
    }

    fun test_enum_vector() {
        let _common_fields = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        let _common_vector_2 = CommonFieldsVector::Bar {
            x: vector[2],
            y: vector[_common_fields]
        };
        let _common_vector_3 = CommonFieldsVector::Bar {
            x: vector[2],
            y: vector[_common_fields]
        };

    }
}
