module 0x42::m {

    fun pure(x: u64): u64 { x + 1 }

    native fun impure_native(x: u64): u64;

    fun impure_borrow(x: u64): u64 {
        let r = &mut x;
        *r = *r + 2;
        x
    }

    fun impure_assign(x: u64): u64 {
        x = x + 1;
        x
    }

    fun impure_indirect(x: u64): u64 {
        impure_borrow(x)
    }

    fun uses_return(x: u64): u64 {
        if (x > 0) return x + 1;
        x
    }

    spec fun can_call_pure(): u64 { pure(1) }

    spec fun cannot_call_impure(x: u64): u64 { impure_borrow(1) }

    spec fun cannot_call_impure_indirect(x: u64): u64 { impure_indirect(1) }

    spec fun cannot_call_impure_assign(x: u64): u64 { impure_assign(x) }

    spec fun cannot_return(): u64 { uses_return(2) }

    fun impure_in_fun_spec(x: u64): u64 {
        x + 1
    }
    spec impure_in_fun_spec {
        ensures result == impure_indirect(x);
    }

    fun impure_in_inline_spec(x: u64): u64 {
        spec {
            assert impure_indirect(x) == 2;
        };
        x + 1
    }

    spec module {
        invariant impure_indirect(22) == 2;
    }
}
