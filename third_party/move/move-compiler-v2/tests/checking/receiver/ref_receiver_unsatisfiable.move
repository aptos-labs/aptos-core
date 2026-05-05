module 0x42::m {

    struct Market { nav: u64 }
    struct Item { size: u64 }

    fun get_nav(self: &Market): u64 { self.nav }
    fun get_size(self: &Item): u64 { self.size }

    inline fun apply<T>(f: |T|u64): u64 {
        abort 0
    }

    // The lambda parameter `x` starts as a fresh type variable T:
    //   *x           adds SomeReference
    //   x.get_nav()  adds SomeReceiverFunction(get_nav)
    //   x.get_size() adds SomeReceiverFunction(get_size) (different name, coexists)
    // SomeReference coexists with the SomeReceiverFunction constraints.
    // No type has both `get_nav` and `get_size` methods, so the compiler should
    // produce a clean "unable to infer" diagnostic rather than the bogus
    // "constraint `&_` incompatible with `fun self.get_nav(...)`".
    fun test(): u64 {
        apply(|x| {
            let _ = *x;
            let _ = x.get_nav();
            x.get_size()
        })
    }
}
