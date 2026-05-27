module 0x42::assign {


    struct Func1<T1, T2, T2>(|T1, T2|T2) has copy, drop;

    enum E2<T1, T1, T2> has drop {
        A {x: T1},
        B {y: T1, x: T2}
    }

    fun local_get_x1<T2, T2>(x1: T2, x2: T2): &T2 {
        &x1
    }

}
