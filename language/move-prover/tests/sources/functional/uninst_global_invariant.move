module 0x42::Test {
    struct S1 has key, store {}

    struct S2<T: store> has key, store { t: T }

    fun foo(account: signer) {
        move_to(&account, S1 {});
    }

    spec module {
        invariant<T> exists<S1>(@0x42) ==> exists<S2<T>>(@0x42);

        // When applying invariant I to function foo, we cannot
        // find a valid instantiation for the type parameter `T`.
        // therefore, this global invariant cannot be checked.
    }
}
