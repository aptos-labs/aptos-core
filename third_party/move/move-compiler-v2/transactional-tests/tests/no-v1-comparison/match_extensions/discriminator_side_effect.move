//# publish
module 0xc0ffee::m {
    fun next(counter: &mut u64): u64 {
        let v = *counter;
        *counter = v + 1;
        v
    }

    public fun test_wildcard_side_effect() {
        let c: u64 = 0;
        let _result = match (next(&mut c)) {
            _ => 99,
        };
        assert!(c == 1);
    }

    public fun test_scalar_single_eval() {
        let c: u64 = 0;
        let _result = match (next(&mut c)) {
            10 => 10,
            20 => 20,
            _ => 99,
        };
        assert!(c == 1);
    }

    public fun test_tuple_single_eval() {
        let c: u64 = 0;
        let _result = match ((next(&mut c), next(&mut c))) {
            (10, 11) => 10,
            (20, 21) => 20,
            _ => 99,
        };
        assert!(c == 2);
    }

    enum Data has drop {
        V1 { f: u64 },
        V2,
    }

    fun make_data(counter: &mut u64): Data {
        *counter = *counter + 1;
        Data::V1 { f: 42 }
    }

    public fun test_mixed_tuple_single_eval() {
        let c: u64 = 0;
        let _result = match ((make_data(&mut c), next(&mut c))) {
            (Data::V1 { f }, 10) => f,
            (Data::V2, 20) => 0,
            _ => 99,
        };
        assert!(c == 2);
    }

    public fun test_mixed_tuple_eval_order() {
        let c: u64 = 0;
        // make_data increments c to 1, then next reads c=1 and increments to 2.
        // So the primitive position should be 1, matching the first arm.
        let result = match ((make_data(&mut c), next(&mut c))) {
            (Data::V1 { f }, 1) => f,
            _ => 0,
        };
        assert!(c == 2);
        assert!(result == 42);
    }
}

//# run 0xc0ffee::m::test_wildcard_side_effect

//# run 0xc0ffee::m::test_scalar_single_eval

//# run 0xc0ffee::m::test_tuple_single_eval

//# run 0xc0ffee::m::test_mixed_tuple_single_eval

//# run 0xc0ffee::m::test_mixed_tuple_eval_order
