module 0xcfff::m {

    struct Test has copy {
        a: u64,
        b: u64
    }

    public fun eq1<T>(x: T, y: T): bool {
        x == y
    }

    public fun eq2<T>(x: T, y: T): bool {
        &x == &y
    }

    public fun eq3<T>(x: vector<T>, y: vector<T>): bool {
        x == y
    }

    public fun eq4(x: Test, y: Test): bool {
        x == y
    }

    public fun neq1<T>(x: T, y: T): bool {
        x != y
    }

    public fun neq2<T>(x: T, y: T): bool {
        &x != &y
    }

    public fun neq3<T>(x: vector<T>, y: vector<T>): bool {
        x != y
    }

    public fun neq4(x: Test, y: Test): bool {
        x != y
    }

    public fun signer_eq(s1: signer, s2: signer): signer {
        if (s1 == s2)
            s1
        else
            s2
    }


}
