module 0x42::simple_recursion {
    struct S {
        f: T
    }
    struct T {
        f: S
    }

    struct S1 {
        f: S2
    }

    struct S2 {
        f: S3
    }

    struct S3 {
        f: S1
    }

    struct S4<T> {
        f: S4<bool>
    }

    struct S5 {
        f: S5
    }

    struct S6 {
        f: S7
    }

    struct S7 {
        f: S7
    }

    struct X {
        f: Y,
        g: Y,
    }

    struct Y {
        f: X,
        g: X
    }
}

module 0x42::type_param {
    struct S {
        f: G<S>
    }

    struct U<T> {
        f: G<U<T>>
    }

    struct G<T> {
        f: T
    }

    struct S1 {
        f: vector<S1>
    }

    struct S2<T1, T2> {
        f: S3<u8, S2<T1, T2>>
    }

    struct S3<T1, T2> {
        f: S2<u8, S3<u8, u8>>
    }

    struct S4<T> {
        f: S4<S4<T>>
    }
}

module 0x42::instantiate_with_self {
    struct S<T> {
        f: T
    }

    struct U {
        // this is ok
        f: S<S<u8>>
    }
}
