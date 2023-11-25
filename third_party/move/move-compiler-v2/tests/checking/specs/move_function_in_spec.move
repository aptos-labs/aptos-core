module 0x42::move_function_in_spec {
    struct TypeInfo has key, copy, drop, store {
        account_address: address,
    }
    public native fun type_of<T>(): TypeInfo;
    public fun change(target: address, new_addr: address): bool acquires TypeInfo {
        let ty = borrow_global_mut<TypeInfo>(target);
        ty.account_address = new_addr;
        true
    }
    public fun no_change(target: address, new_addr: address): bool acquires TypeInfo {
        let ty = borrow_global<TypeInfo>(target);
        ty.account_address == new_addr
    }

    fun foo<T>() {
        let type_info = type_of<T>();
        let account_address = type_info.account_address;
        spec {
            assert change(account_address, account_address);
            assert no_change(account_address, account_address);
            assert account_address == type_of<T>().account_address;
        };
    }
}
