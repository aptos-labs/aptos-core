module 0x815::m {

    enum CommonFields has copy, drop {
        Foo{x: u64, y: u8},
        Bar{x: u64, y: u8, z: u32}
    }

    spec CommonFields {
        invariant self.x > 20;
        invariant (self is CommonFields::Bar) ==> self.z > 10;
    }

    fun t9_common_field(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        common.x = 15; // struct invariant fails
        common.x
    }

    fun test_data_invariant() {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        let CommonFields::Bar {x: _x, y: _y, z} = &mut common;
        *z = 9; // struct invariant fails
    }

    fun test_match_ref(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        match (&common) {
            Foo {x, y: _} => *x,
            Bar {x, y: _, z: _ } => *x + 1
        }
    }

    spec test_match_ref {
        ensures result == 31;
    }

    enum CommonFieldsVector has drop {
        Foo{x: vector<u8>},
        Bar{x: vector<u8>, y: vector<CommonFields>}
    }

    fun test_enum_vector() {
        let _common_vector_1 = CommonFieldsVector::Foo {
            x: vector[2]
        };
        let _common_fields = CommonFields::Bar {
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
            assert _common_vector_2.y[0] == CommonFields::Bar {
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
