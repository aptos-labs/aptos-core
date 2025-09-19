module 0x99::m {
    use std::option;
    struct OptionStore<phantom T> has copy,drop,store {
        o: u64,
    }
    struct FunctionStore has key {
        f: ||u64 has copy+drop+store,
    }
    public fun id(x: OptionStore<option::Option<u64>>): u64 {
        x.o
    }
    entry fun entry_func() {
        let v = OptionStore { o: 3 };
        let _f: ||u64 has copy+drop+store = || id(v);
    }
}
