module aptos_std::type_info {
    use std::string;

    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }

    public fun account_address(type_info: &TypeInfo): address {
        type_info.account_address
    }

    public fun module_name(type_info: &TypeInfo): vector<u8> {
        type_info.module_name
    }

    public fun struct_name(type_info: &TypeInfo): vector<u8> {
        type_info.struct_name
    }

    public native fun type_of<T>(): TypeInfo;
    public native fun full_name<T>(): string::String;

    #[test]
    fun test() {
        let type_info = type_of<TypeInfo>();
        assert!(account_address(&type_info) == @aptos_std, 0);
        assert!(module_name(&type_info) == b"type_info", 1);
        assert!(struct_name(&type_info) == b"TypeInfo", 2);
    }

    #[test]
    fun test_full_name() {
        use aptos_std::table::Table;

        assert!(full_name<bool>() == string::utf8(b"bool"), 0);
        assert!(full_name<u8>() == string::utf8(b"u8"), 1);
        assert!(full_name<u64>() == string::utf8(b"u64"), 2);
        assert!(full_name<u128>() == string::utf8(b"u128"), 3);
        assert!(full_name<address>() == string::utf8(b"address"), 4);
        assert!(full_name<signer>() == string::utf8(b"signer"), 5);

        // vector
        assert!(full_name<vector<u8>>() == string::utf8(b"vector<u8>"), 6);
        assert!(full_name<vector<vector<u8>>>() == string::utf8(b"vector<vector<u8>>"), 7);
        assert!(full_name<vector<vector<TypeInfo>>>() == string::utf8(b"vector<vector<0x1::type_info::TypeInfo>>"), 8);


        // struct
        assert!(full_name<TypeInfo>() == string::utf8(b"0x1::type_info::TypeInfo"), 9);
        assert!(full_name<
            Table<
                TypeInfo,
                Table<u8, vector<TypeInfo>>
            >
        >() == string::utf8(b"0x1::table::Table<0x1::type_info::TypeInfo, 0x1::table::Table<u8, vector<0x1::type_info::TypeInfo>>"), 10);
    }
}
