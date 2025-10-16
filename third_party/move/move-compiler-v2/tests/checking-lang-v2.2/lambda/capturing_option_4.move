module 0x99::m {
    use std::option;
    struct Store<T: store+drop+copy> has copy,drop,store {
        o: T,
    }
    struct FunctionStore has key {
        f: ||option::Option<u64> has copy+drop+store,
    }
    public fun id<T: store+drop+copy>(x: Store<T>): T {
        x.o
    }
    fun init_module(account: &signer) {
        let v = Store { o: option::none() };
        let f: ||option::Option<u64> has copy+drop+store = || id(v);
        move_to(account, FunctionStore { f });
    }
    entry fun entry_func() {
        let v = Store { o: option::none() };
        let _f: ||option::Option<u64> has copy+drop+store = || id(v);
    }
 }
