module 0x1::string_utils_test {
    use std::string;
    use aptos_std::string_utils;

    struct Test has copy, drop {
        x: u64,
    }

    enum TestEnum has copy, drop {
        V1 { x: u64, },
        V2 { x: u64, y: Test },
    }

    public fun dummy(_x: &u64, _v: u64) { }

    public fun test1() {}

    public fun test2(_a: u64, _b: u16, _c: address, _d: vector<Test>) {}

    public fun test3(f: |&u64|, x: u64) { f(&x) }

    public fun test4<A: drop, B: drop, C: drop, D>(_x: A, _y: B, _z: C) {}

    public entry fun run_all() {

        // === Lambda lifting === //

        let f1: || has drop = || {};
        assert_eq(&f1, b"0x1::string_utils_test::__lambda__1__run_all()", 1);

        let f2: |u8, u8| has drop = |a, b| {
            let e = TestEnum::V2 { x: 10, y: Test { x: 20 } };
            test4<u8, TestEnum, u8, u8>(a, e, b)
        };
        assert_eq(&f2, b"0x1::string_utils_test::__lambda__2__run_all()", 2);

        // === No capturing === //

        let f3: || has drop = test1;
        assert_eq(&f3, b"0x1::string_utils_test::test1()", 3);

        let f4: |u64, u16, address, vector<Test>| has drop = test2;
        assert_eq(&f4, b"0x1::string_utils_test::test2()", 4);

        let f5: |(|&u64|), u64| has drop = test3;
        assert_eq(&f5, b"0x1::string_utils_test::test3()", 5);

        // === Capturing simple === //

        let f6: |u64, vector<Test>| has drop = |a, b| test2(a, 20, @0x123, b);
        assert_eq(&f6, b"0x1::string_utils_test::test2(_, 20, @0x123, ..)", 6);

        let v = vector[Test { x: 1 }, Test { x: 2 }];
        let f7: |u16| has drop = |a| test2(10, a, @0x123, v);
        assert_eq(&f7, b"0x1::string_utils_test::test2(10, _, @0x123, [ { 1 }, { 2 } ], ..)", 7);

        // === With type arguments === //

        let f8: |u64, Test, (|u64| has drop)| has drop = test4<u64, Test, (|u64| has drop), TestEnum>;
        assert_eq(
          &f8,
          b"0x1::string_utils_test::test4<u64, 0x1::string_utils_test::Test, |u64|() has drop, 0x1::string_utils_test::TestEnum>()",
          8,
        );

        let e = TestEnum::V2 { x: 10, y: Test { x: 20 } };
        let f9: |u64, u8| has drop = |a, b| test4<u64, TestEnum, u8, u8>(a, e, b);
        assert_eq(
            &f9,
            b"0x1::string_utils_test::test4<u64, 0x1::string_utils_test::TestEnum, u8, u8>(_, #1{ 10, { 20 } }, ..)",
            9,
        );

        let h1: |&u64| has drop = |x| dummy(x, 10);
        let h2: || has drop = || test3(h1, 30);
        assert_eq(
            &h2,
            b"0x1::string_utils_test::test3(0x1::string_utils_test::dummy(_, 10, ..), 30, ..)",
            10,
        );
    }

    public fun assert_eq<T>(x: &T, expected: vector<u8>, abort_code: u64) {
        let actual = string_utils::to_string(x);
        let expected = string::utf8(expected);
        assert!(actual == expected, abort_code);
    }
}
