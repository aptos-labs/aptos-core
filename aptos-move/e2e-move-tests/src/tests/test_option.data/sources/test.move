module account::test_option {
    use std::option;

    struct FunctionStore has key {
        f: ||R has copy+drop+store,
    }

    struct R(u64) has copy, drop, key, store;

    public fun id(x: R): R {
        x
    }

    struct R2<T: copy + drop +store>(T) has copy, drop, key, store;

    fun init_module(account: &signer) {
        let v = R(1);
        let v2 = R2(option::none<u64>());
        let v3 = R2(option::some<u32>(1));
        let f: ||R has copy+drop+store = || id(v);
        move_to(account, FunctionStore { f });
        move_to(account, v2);
        move_to(account, v3);
    }

    entry fun entry_function(account: &signer, o: u128) {
        bcs_bool();
        let v = R2(option::some(o));
        move_to(account, v);
    }

    fun bcs_bool() {
        use std::bcs;
        let expected_bytes = x"01";
        let actual_bytes = bcs::to_bytes(&true);
        assert!(actual_bytes == expected_bytes, 0);

        let expected_size = actual_bytes.length();
        let actual_size = bcs::serialized_size(&true);
        assert!(actual_size == expected_size, 1);

        assert!(option::some(actual_size) == bcs::constant_serialized_size<bool>(), 2);
    }

    #[view]
    public fun get_option(addr: address): option::Option<u128> {
        let v = borrow_global<R2<option::Option<u128>>>(addr);
        v.0
    }
}
