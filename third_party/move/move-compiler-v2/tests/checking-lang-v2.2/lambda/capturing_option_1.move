module 0x99::m {
    use std::option;
    struct FunctionStore has key {
        f: ||option::Option<u64> has copy+drop+store,
    }
    public fun id(x: option::Option<u64>): option::Option<u64> {
        x
    }
    fun init_module(account: &signer) {
        let v = option::none();
        let f: ||option::Option<u64> has copy+drop+store = || id(v);
        move_to(account, FunctionStore { f });
    }
    entry fun entry_func() {
        let v = option::none();
        let _f: ||option::Option<u64> has copy+drop+store = || id(v);
    }
}
