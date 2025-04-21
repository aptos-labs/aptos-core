module 0x815::m {

    enum CommonFields has copy, drop {
        Foo{x: u64, y: u8},
        Bar{x: u64, y: u8, z: u32}
    }


    fun select_abort(): u32 {
        let common = CommonFields::Foo {
            x: 30,
            y: 40,
        };
        common.z // aborts
    }

    spec select_abort {
        aborts_if false;
    }

    fun unpack_abort(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        let CommonFields::Foo {x, y: _y} = common; // aborts
        x
    }

    spec unpack_abort {
        aborts_if false;
    }

    fun test_match(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        match (common) {
            Foo {x, y: _} => x,
            Bar {x, y: _, z: _ } => x + 1
        }
    }

    spec test_match {
        aborts_if false;
    }

    fun test_borrow_field_abort() {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        let CommonFields::Foo {x, y: _y} = &mut common; // aborts
        *x = 20;
    }

    spec test_borrow_field_abort {
        aborts_if false;
    }

    fun test_borrow_field_not_abort() {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        let CommonFields::Bar {x, y: _y, z: _z} = &mut common;
        *x = 20;
    }

    spec test_borrow_field_not_abort {
        aborts_if false;
    }

    fun test_borrow_field_not_abort_2() {
        let common = CommonFields::Foo {
            x: 30,
            y: 40,
        };
        let CommonFields::Foo {x, y: _y} = &mut common;
        *x = 20;
    }

    spec test_borrow_field_not_abort_2 {
        aborts_if false;
    }

    fun test_get_field_abort(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        let CommonFields::Foo {x, y: _y} = & common; // aborts
        *x
    }

    spec test_get_field_abort {
        aborts_if false;
    }

    enum CommonFields2 has drop {
        Foo{x: vector<u8>},
        Bar{x: vector<u8>, y: CommonFields}
    }

    fun test_match_abort(): u64 {
        let common = CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
        let _common_vector_2 = CommonFields2::Bar {
            x: vector[2],
            y: common
        };
        match (_common_vector_2) {
            CommonFields2::Bar {x:_x, y: CommonFields::Bar {x:_, y:_, z:_z} } if _x[0] > 80 || _z <= 50 => {
                abort 30 // aborts
            }
            _ => 2
        }
    }

    spec test_match_abort {
        aborts_if false;
    }


}
