module account::test {

    struct FunctionStore has key {
        f: ||R has copy+drop+store,
    }

    struct R(u64) has copy, drop, key, store;

    public fun id(x: R): R {
        x
    }

    fun init_module(account: &signer) {
        let v = R(1);
        let f: ||R has copy+drop+store = || id(v);
        move_to(account, FunctionStore { f });
    }
}
