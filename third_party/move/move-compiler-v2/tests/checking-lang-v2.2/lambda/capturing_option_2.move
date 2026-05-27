module 0x99::m {
    use std::option;
    struct OptionStore has copy,drop,store {
        o: option::Option<u64>,
    }
    struct FunctionStore has key {
        f: ||option::Option<u64> has copy+drop+store,
    }
    public fun id(x: OptionStore): option::Option<u64> {
        x.o
    }
    entry fun entry_func() {
        let v = OptionStore { o: option::none() };
        let _f: ||option::Option<u64> has copy+drop+store = || id(v);
    }
}
