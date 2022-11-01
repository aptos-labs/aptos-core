module aptos_std::type_info {
    use std::string;
    use std::features;

    //
    // Error codes
    //

    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 1;

    //
    // Structs
    //

    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }

    //
    // Public functions
    //

    public fun account_address(type_info: &TypeInfo): address {
        type_info.account_address
    }

    public fun module_name(type_info: &TypeInfo): vector<u8> {
        type_info.module_name
    }

    public fun struct_name(type_info: &TypeInfo): vector<u8> {
        type_info.struct_name
    }

    /// Returns the current chain ID, mirroring what `aptos_framework::chain_id::get()` would return, except in `#[test]`
    /// functions, where this will always return `4u8` as the chain ID, whereas `aptos_framework::chain_id::get()` will
    /// return whichever ID was passed to `aptos_framework::chain_id::initialize_for_test()`.
    public fun chain_id(): u8 {
        if (!features::aptos_stdlib_chain_id_enabled()) {
            abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
        };

        chain_id_internal()
    }

    public native fun type_of<T>(): TypeInfo;

    public native fun type_name<T>(): string::String;

    native fun chain_id_internal(): u8;

    #[test]
    fun test() {
        let type_info = type_of<TypeInfo>();
        assert!(account_address(&type_info) == @aptos_std, 0);
        assert!(module_name(&type_info) == b"type_info", 1);
        assert!(struct_name(&type_info) == b"TypeInfo", 2);
    }

    #[test(fx = @std)]
    fun test_chain_id(fx: signer) {
        // We need to enable the feature in order for the native call to be allowed.
        features::change_feature_flags(&fx, vector[features::get_aptos_stdlib_chain_id_feature()], vector[]);

        // The testing environment chain ID is 4u8.
        assert!(chain_id() == 4u8, 1);
    }

    #[test]
    fun test_type_name() {
        use aptos_std::table::Table;

        assert!(type_name<bool>() == string::utf8(b"bool"), 0);
        assert!(type_name<u8>() == string::utf8(b"u8"), 1);
        assert!(type_name<u64>() == string::utf8(b"u64"), 2);
        assert!(type_name<u128>() == string::utf8(b"u128"), 3);
        assert!(type_name<address>() == string::utf8(b"address"), 4);
        assert!(type_name<signer>() == string::utf8(b"signer"), 5);

        // vector
        assert!(type_name<vector<u8>>() == string::utf8(b"vector<u8>"), 6);
        assert!(type_name<vector<vector<u8>>>() == string::utf8(b"vector<vector<u8>>"), 7);
        assert!(type_name<vector<vector<TypeInfo>>>() == string::utf8(b"vector<vector<0x1::type_info::TypeInfo>>"), 8);


        // struct
        assert!(type_name<TypeInfo>() == string::utf8(b"0x1::type_info::TypeInfo"), 9);
        assert!(type_name<
            Table<
                TypeInfo,
                Table<u8, vector<TypeInfo>>
            >
        >() == string::utf8(b"0x1::table::Table<0x1::type_info::TypeInfo, 0x1::table::Table<u8, vector<0x1::type_info::TypeInfo>>>"), 10);
    }

    #[verify_only]
    fun verify_type_of() {
        let type_info = type_of<TypeInfo>();
        let account_address = account_address(&type_info);
        let module_name = module_name(&type_info);
        let struct_name = struct_name(&type_info);
        spec {
            assert account_address == @aptos_std;
            assert module_name == b"type_info";
            assert struct_name == b"TypeInfo";
        };
    }

    #[verify_only]
    fun verify_type_of_generic<T>() {
        let type_info = type_of<T>();
        let account_address = account_address(&type_info);
        let module_name = module_name(&type_info);
        let struct_name = struct_name(&type_info);
        spec {
            assert account_address == type_of<T>().account_address;
            assert module_name == type_of<T>().module_name;
            assert struct_name == type_of<T>().struct_name;
        };
    }
}
