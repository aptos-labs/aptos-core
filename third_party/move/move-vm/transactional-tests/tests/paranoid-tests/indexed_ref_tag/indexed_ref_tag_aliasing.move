//# publish
module 0xc0ffee::corner_cases {
    enum Foo has copy, drop {
        A {
            x: u64,
        },
        B {
            x: u64,
        },
    }

    public fun fun1(): u64 {
        let foo = Foo::A { x: 0 };
        let r2 = &mut foo;
        let r1 = &mut foo;
        *r2 = Foo::B { x: 5 };
        r1.x
    }

    fun update(foo: &mut Foo) {
        *foo = Foo::B { x: 5 };
    }

    fun get_value(foo: &Foo): u64 {
        foo.x
    }

    public fun fun2(): u64 {
        let foo = Foo::A { x: 0 };
        let r2 = &mut foo;
        let r1 = &mut foo;
        update(r2);
        get_value(r1)
    }

    public fun fun3(): u64 {
        let foo = Foo::A { x: 0 };
        let r2 = &mut foo;
        let r1 = r2;
        *r1 = Foo::B { x: 5 };
        r2.x
    }

    public fun fun4(): u64 {
        let foo = Foo::A { x: 0 };
        let r2 = &mut foo;
        let r1 = &mut foo;
        let r3 = &mut foo;
        *r2 = Foo::B { x: 5 };
        r1.x + r3.x
    }

    public fun fun5(): u64 {
        let foo = Foo::A { x: 0 };
        let r2 = &mut foo.x;
        let r1 = r2;
        *r1 = 5;
        *r2
    }

    public fun fun6(): u64 {
        let foo = Foo::A { x: 0 };
        let r2 = &mut foo;
        let r1 = r2;
        *r1 = Foo::B { x: 5 };
        r2.x
    }
}

//# run 0xc0ffee::corner_cases::fun1 --verbose

//# run 0xc0ffee::corner_cases::fun2 --verbose

//# run 0xc0ffee::corner_cases::fun3 --verbose

//# run 0xc0ffee::corner_cases::fun4 --verbose

//# run 0xc0ffee::corner_cases::fun5 --verbose

//# run 0xc0ffee::corner_cases::fun6 --verbose
