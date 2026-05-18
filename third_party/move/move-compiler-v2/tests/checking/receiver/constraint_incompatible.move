module 0x42::m {

    struct Position { is_isolated: bool }
    struct Market { nav: u64 }

    fun get_nav(self: &Market): u64 {
        self.nav
    }

    inline fun apply<T>(f: |&T|u64): u64 {
        abort 0
    }

    // The lambda parameter type is unconstrained. The field access `x.is_isolated`
    // adds a SomeStruct{is_isolated} constraint, and the receiver call `x.get_nav()`
    // adds a SomeReceiverFunction constraint. These coexist on the type variable,
    // but no struct in scope satisfies both, so the type cannot be inferred.
    fun test(): u64 {
        apply(|x| {
            let _ = x.is_isolated;
            x.get_nav()
        })
    }
}
