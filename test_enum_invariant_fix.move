module test::enum_invariant_test {
    use std::debug;

    struct FirstStruct {
        fsnum: u64,
    }

    struct SecondStruct {
        ssnum: u64,
    }

    enum Foobar {
        Jake { foo: FirstStruct },
        Silverman { bar: SecondStruct },
    }

    spec FirstStruct {
        invariant self.fsnum >= 666;
    }

    spec SecondStruct {
        invariant self.ssnum <= 999;
    }

    public fun test_jake_variant() {
        let foo = FirstStruct { fsnum: 999 };
        let foobar = Foobar::Jake { foo };

        // This should only check FirstStruct invariants, not SecondStruct
        let foo_ref = &mut foobar.foo;
        foo_ref.fsnum = 1000;

        debug::print(&foobar);
    }

    public fun test_silverman_variant() {
        let bar = SecondStruct { ssnum: 777 };
        let foobar = Foobar::Silverman { bar };

        // This should only check SecondStruct invariants, not FirstStruct
        let bar_ref = &mut foobar.bar;
        bar_ref.ssnum = 888;

        debug::print(&foobar);
    }
}
