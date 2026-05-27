module 0x99::m {
    use std::option;
    use std::vector;
    struct FunctionStore has key {
        f: ||option::Option<u64> has copy+drop+store,
    }
    public fun id(_x: vector<option::Option<u64>>): option::Option<u64> {
        option::none()
    }
    entry fun entry_func() {
        let v = vector::empty();
        let _f: ||option::Option<u64> has copy+drop+store = || id(v);
    }
}
