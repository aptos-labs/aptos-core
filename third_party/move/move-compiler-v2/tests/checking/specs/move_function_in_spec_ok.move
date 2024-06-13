module 0x42::move_function_in_spec {
    struct TypeInfo has key, copy, drop, store {
        account_address: address,
    }

    public fun type_of<T>(): TypeInfo {
        abort 1
    }

    public fun no_change(target: address, new_addr: address): bool acquires TypeInfo {
        let ty = borrow_global<TypeInfo>(target);
        ty.account_address == new_addr
    }

    fun foo<T>() {
        let type_info = type_of<T>();
        let account_address = type_info.account_address;
        spec {
            assert no_change(account_address, account_address);
            assert account_address == type_of<T>().account_address;
        };
    }
}
