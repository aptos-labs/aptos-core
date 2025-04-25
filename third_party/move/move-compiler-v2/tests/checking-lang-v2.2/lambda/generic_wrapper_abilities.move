// Fixes ##16158
module 0xc0ffee::m {
    struct Func<T>(|T|T) has copy;

    fun apply<T>(f: Func<T>, v: T): T {
        let Func(f) = f;
        f(v)
    }

    struct S {
        x: u64,
    }

    fun merge(self: S, other: S): S {
        let S { x: self_x } = self;
        let S { x: other_x } = other;
        S { x: self_x + other_x }
    }

    fun test(): S {
        let s = S { x: 42 };
        // We expect an error here since the function type in `Func<S>` continues
        // to have the `copy` requirement even though `S` is not. This is related
        // to the current general mismatch of abilities and function types in
        // generic structs.
        let f: Func<S> = |x| { x.merge(s) };
        apply(f, S {x: 0})
    }
}
