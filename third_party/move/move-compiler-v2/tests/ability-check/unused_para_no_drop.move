module 0xc0ffee::m {

    struct S {
    }

    struct G has drop {
    }

    public fun f1<T>(_x: T) {
    }

    public fun f2<T>(_x: &T) {
    }

    public fun f3(_x: S) {
    }

    public fun f4(_x: &S) {
    }

    public fun f5(_x: vector<S>) {
    }

    public fun f6(_x: G) {
    }

    public fun f7(_x: &G) {
    }

    public fun f8(_x: u64) {
    }

    public fun f9<T>(_x: T) {
        abort 0 // no error for this function
    }

    public fun f10<T>(x: T, _y:T): T {
        x
    }

    public fun f11(x: S, y: S): bool {
        &x == &y
    }

    public fun f12<T>(x: T, y: T): bool {
        &x == &y
    }

    struct S2 {
        foo: u64
    }

    public fun f13(x: S2, y: S2): bool {
        x.foo == y.foo
    }

}
