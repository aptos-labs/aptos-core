module AptosFramework::TypeInfo {
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

    #[test]
    fun test() {
        let type_info = type_of<AptosFramework::Account::Account>();
        assert!(account_address(&type_info) == @AptosFramework, 0);
        assert!(module_name(&type_info) == b"Account", 1);
        assert!(struct_name(&type_info) == b"Account", 2);
    }
}
