script {
    use 0x1::string_utils_test::{assert_eq, Test};

    fun main() {
        let f1: || has drop = 0x1::string_utils_test::test1;
        assert_eq(&f1, b"0x1::string_utils_test::test1()", 1);

        let f2: |u64, vector<Test>| has drop = |a, b| 0x1::string_utils_test::test2(a, 20, @0x123, b);
        assert_eq(&f2, b"0x1::string_utils_test::test2(_, 20, @0x123, ..)", 2);
    }
}
