module 0xcfff::m {

    fun foo <T: copy + drop>(x: T, y: T): T {
        if (x==y)
            x
        else
            y
    }
     public fun eq1<T: copy + drop>(x: T, y: T): bool {
        foo(x, y) == foo(y, x)
    }

    public fun eq2<T: copy + drop>(x: T, y: T, z: bool): bool {
        &(&x == &y) == &z
    }

    public fun eq3<T: copy + drop>(x: T, y: T, z: bool): bool {
        x == y == z
    }

    public fun eq4<T: copy + drop>(x: T, y: T, z: bool): bool {
        &x == &y == z
    }

    struct Test has copy, drop {
        a: u64,
        b: u64
    }

    struct Test1 has copy, drop {
        a: Test,
        b: u64
    }

    public fun eq5(x: Test1, y: Test1): bool {
        x.a == y.a
    }

    public fun eq6(x: Test1, y: Test1): bool {
        &x.a == &y.a
    }

     public fun eq7(x: Test1, y: Test1): bool {
        let s1 = &x;
        let s2 = &y;
        *s1 == *s2
    }
}
