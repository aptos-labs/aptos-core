// note: intentionally using 0xa here to test non-0x1 module addresses
module 0xA::type_name_tests {
    #[test_only]
    use std::type_name::{get, into_string};
    #[test_only]
    use std::ascii::string;

    struct TestStruct {}

    struct TestGenerics<phantom T> { }

    #[test]
    fun test_ground_types() {
        assert!(into_string(get<u8>()) == string(b"u8"), 0);
        assert!(into_string(get<u64>()) == string(b"u64"), 0);
        assert!(into_string(get<u128>()) == string(b"u128"), 0);
        assert!(into_string(get<address>()) == string(b"address"), 0);
        assert!(into_string(get<signer>()) == string(b"signer"), 0);
        assert!(into_string(get<vector<u8>>()) == string(b"vector<u8>"), 0)
    }

    #[test]
    fun test_structs() {
        assert!(into_string(get<TestStruct>()) == string(b"0xa::type_name_tests::TestStruct"), 0);
        assert!(into_string(get<std::ascii::String>()) == string(b"0x1::ascii::String"), 0);
        assert!(into_string(get<std::option::Option<u64>>()) == string(b"0x1::option::Option<u64>"), 0);
        assert!(into_string(get<std::string::String>()) == string(b"0x1::string::String"), 0);
    }

    #[test]
    fun test_generics() {
        assert!(into_string(get<TestGenerics<std::string::String>>()) == string(b"0xa::type_name_tests::TestGenerics<0x1::string::String>"), 0);
        assert!(into_string(get<vector<TestGenerics<u64>>>()) == string(b"vector<0xa::type_name_tests::TestGenerics<u64>>"), 0);
        assert!(into_string(get<std::option::Option<TestGenerics<u8>>>()) == string(b"0x1::option::Option<0xa::type_name_tests::TestGenerics<u8>>"), 0);
    }
}
