module 0x42::m {
    // Struct used as return type
    struct Result has drop { value: u64 }

    // Struct used as parameter type
    struct Input has drop { x: u64 }

    // Struct used as field type
    struct Outer has drop { inner: Inner }
    struct Inner has drop { value: u64 }

    // Struct used in global storage operations
    struct Resource has key { data: u64 }

    // Struct used only in function type
    struct FunArg has drop, copy { val: u64 }
    struct FunResult has drop { res: u64 }

    fun make_result(): Result {
        Result { value: 42 }
    }

    fun process_input(input: Input): u64 {
        input.x
    }

    fun make_outer(): Outer {
        Outer { inner: Inner { value: 1 } }
    }

    fun apply_fun(f: |FunArg|FunResult, arg: FunArg): FunResult {
        f(arg)
    }

    public fun test(account: &signer): u64 {
        let r = make_result();
        let i = Input { x: 10 };
        let o = make_outer();
        move_to(account, Resource { data: r.value });
        let res = apply_fun(|a| FunResult { res: a.val }, FunArg { val: 5 });
        process_input(i) + o.inner.value + res.res
    }
}
