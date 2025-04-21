//# publish
module 0xc0ffee::m {
    enum Foo {
        A(Bar)
    } has drop;

    enum Bar {
        B(Foo)
    } has drop;

    enum Blah {
        A(u64),
        B(Foo),
    } has drop;

    fun test(x: Blah) {
        match (x) {
            Blah::A(_) => (),
            Blah::B(Foo::A(_)) => (),
        }
    }

    fun main() {
        let a = Blah::A(42);
        test(a);
    }
}

//# run 0xc0ffee::m::main
