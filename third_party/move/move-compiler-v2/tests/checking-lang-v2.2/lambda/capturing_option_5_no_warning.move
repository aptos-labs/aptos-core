module 0x99::m {
    use std::option;
    struct FunctionStore has key {
        f: ||u64 has copy+drop+store,
    }
    public fun id(f: |option::Option<u64>| u64 has copy+drop+store): u64 {
        f(option::some(3))
    }
    public fun id2(_v: option::Option<u64>): u64 {
        3
    }
    fun init_module(account: &signer) {
        let v :|option::Option<u64>|u64 has copy+drop+store = id2;
        let f: ||u64 has copy+drop+store = || id(v);
        move_to(account, FunctionStore { f });
    }
    entry fun entry_func() {
        let v :|option::Option<u64>|u64 has copy+drop+store = id2;
        let _f: ||u64 has copy+drop+store = || id(v);
    }
}
