// Module-level `#[lint::skip(unreachable_code)]` should suppress the lint
// on every function in the module.
#[lint::skip(unreachable_code, needless_return)]
module 0xc0ffee::m {
    public fun a(): u64 {
        abort 0;
        42
    }

    public fun b(x: u64): u64 {
        return x;
        x + 1
    }
}
