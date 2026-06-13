module account::test {
    use std::option;

    struct FunctionStore has key {
        f: ||R has copy+drop+store,
    }

    struct R has copy, drop, key, store {
        value: u64,
        extra: u64,
    }

    public fun id(x: R): R {
        x
    }

    struct R2<T: copy + drop +store>(T) has copy, drop, key, store;

    fun init_module(account: &signer) {
        let v = R { value: 1, extra: 2 };
        let v2 = R2(option::none<u64>());
        let v3 = R2(option::some<u32>(1));
        let f: ||R has copy+drop+store = || id(v);
        move_to(account, FunctionStore { f });
        move_to(account, v2);
        move_to(account, v3);
    }

    entry fun entry_function(account: &signer, o: option::Option<u128>) {
        let v = R2(o);
        move_to(account, v);
    }
}
