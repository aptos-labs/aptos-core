module aptos_framework::type_info {
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

    /// Return `true` if `type_info_1` and `type_info_2` are the same,
    /// else `false`
    public fun are_same_type_info(
        type_info_1: &TypeInfo,
        type_info_2: &TypeInfo
    ): bool {
        type_info_1.account_address == type_info_2.account_address &&
        type_info_1.module_name == type_info_2.module_name &&
        type_info_1.struct_name == type_info_2.struct_name
    }

    #[test]
    fun test() {
        let type_info = type_of<TypeInfo>();
        assert!(account_address(&type_info) == @aptos_framework, 0);
        assert!(module_name(&type_info) == b"type_info", 1);
        assert!(struct_name(&type_info) == b"TypeInfo", 2);
    }

    #[test]
    fun test_are_same_type_info() {
        let type_info = type_of<TypeInfo>();
        // Verify same type infos assesed as such
        assert!(are_same_type_info(&type_info, &type_info), 0);
        let type_info_2 = copy type_info; // Copy reference type info
        // Assign a different struct name to copy
        type_info_2.struct_name = b"DifferentType";
        // Verify different type infos from same module assessed as such
        assert!(!are_same_type_info(&type_info, &type_info_2), 1);
        let type_info_3 = copy type_info; // Copy reference type info
        // Assign a different module name to copy
        type_info_3.module_name = b"different_module";
        // Verify false return when only module name different
        assert!(!are_same_type_info(&type_info, &type_info_3), 2);
        let type_info_4 = copy type_info; // Copy reference type info
        // Assign a different account address to copy
        type_info_4.account_address = @core_resources;
        // Verify false return when only account address different
        assert!(!are_same_type_info(&type_info, &type_info_4), 3);
    }
}
