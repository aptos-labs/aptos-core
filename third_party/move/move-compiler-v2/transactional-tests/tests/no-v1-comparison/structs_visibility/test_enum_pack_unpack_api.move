//# publish
module 0x42::m1 {


    public enum Result<T: copy + drop, E: copy +drop> has copy, drop {
        Ok(T),
        Err(E)
    }


    public enum E has drop {
        A {x: u64},
        B {x: u64}
    }

    public enum Inner {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }


}

//# publish
module 0x42::m2 {

    use 0x42::m1::Result;
    use 0x42::m1::E;
    use 0x42::m1::Inner;

    public fun test_pack_unpack_result_ok() {
        let result = Result::Ok<u64, u64>(42);
        assert!(result == Result::Ok(42), 1);
        let Result::Ok(ok) = result;
        assert!(ok == 42, 2);
    }

    public fun test_pack_unpack_result_err() {
        let result: Result<u64, u8> = Result::Err(7);
        let Result::Err(err) = result;
        assert!(err == 7, 3);
    }

    public fun test_unpack_enum_e() {
        let a = E::A { x: 100 };
        let b = E::B { x: 200 };

        let E::A { x } = a;
        assert!(x == 100, 4);

        let E::B { x: y } = b;
        assert!(y == 200, 5);
    }

    public fun test_unpack_inner() {
        let v1 = Inner::Inner1 { x: 10 };
        let v2 = Inner::Inner2 { x: 20, y: 30 };

        let val1 = match (v1) {
            Inner::Inner1 { x } => x,
            Inner::Inner2 { x, y } => x + y
        };
        let val2 = match (v2) {
            Inner::Inner1 { x } => x,
            Inner::Inner2 { x, y } => x + y
        };
        assert!(val1 == 10, 6);
        assert!(val2 == 50, 7);
    }


}

//# run 0x42::m2::test_pack_unpack_result_ok

//# run 0x42::m2::test_pack_unpack_result_err

//# run 0x42::m2::test_unpack_enum_e

//# run 0x42::m2::test_unpack_inner
