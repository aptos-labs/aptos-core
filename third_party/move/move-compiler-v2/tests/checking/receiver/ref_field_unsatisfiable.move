module 0x42::m {

    struct Position { is_isolated: bool }
    struct Bond { duration: u64 }

    inline fun apply<T>(f: |T|u64): u64 {
        abort 0
    }

    // The lambda parameter `x` starts as a fresh type variable T:
    //   *x            adds SomeReference
    //   x.is_isolated adds SomeStruct{is_isolated}
    //   x.duration    adds SomeStruct{duration} (merged with the above)
    // SomeReference coexists with the merged SomeStruct{is_isolated, duration}
    // constraint. No struct in scope has both fields, so the compiler should
    // produce a clean "unable to infer" diagnostic rather than the bogus
    // "constraint `&_` incompatible with
    // `struct{...}`".
    fun test(): u64 {
        apply(|x| {
            let _ = *x;
            let _ = x.is_isolated;
            let _ = x.duration;
            0
        })
    }
}
