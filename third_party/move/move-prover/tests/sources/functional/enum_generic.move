module 0x815::m {

    enum CommonFields<U> has copy, drop {
        Foo{x: U, y: u8},
        Bar{x: U, y: u8, z: u32}
    }

    enum CommonFieldsVector<T, U> has drop {
        Foo{x: vector<T>},
        Bar{x: vector<T>, y: vector<CommonFields<U>>}
    }

    fun test_enum_vector() {
        let _common_vector_1 = CommonFieldsVector::Foo<u64, u64> {
            x: vector[2]
        };
        let _common_fields = CommonFields::Bar<u64> {
            x: 30,
            y: 40,
            z: 50
        };
        let _common_vector_2 = CommonFieldsVector::Bar {
            x: vector[2],
            y: vector[_common_fields]
        };
        spec {
            assert _common_vector_1.x != _common_vector_2.x; // this fails
            assert _common_vector_2.y[0] == CommonFields::Bar<u64> {
                x: 30,
                y: 40,
                z: 50
            };
        };
        let _common_vector_3 = CommonFieldsVector::Bar {
            x: vector[2],
            y: vector[_common_fields]
        };
        spec {
            assert _common_vector_2.x == _common_vector_3.x;
            assert _common_vector_2 == _common_vector_3;
        };

    }

}
