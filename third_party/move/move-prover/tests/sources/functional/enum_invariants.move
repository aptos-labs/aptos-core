module 0x42::m {

    struct Foo {
        foo: u64,
    }

    struct Bar {
        bar: u64,
    }

    spec Bar{
        invariant self.bar > 0;
    }

    enum Foobar {
        First {
            foo: Foo,
        },
        Second {
            bar: Bar,
        }
    }

    fun add_one_if_first_ok(self: &mut Foobar) {
        match(self) {
            Foobar::First { foo } => foo.foo = foo.foo + 1,
            Foobar::Second { bar } => bar.bar = bar.bar + 1,
        }
    }

    fun sub_one_if_second_fail(self: &mut Foobar) {
        match(self) {
            Foobar::First { foo } => foo.foo = foo.foo + 1,
            Foobar::Second { bar } => bar.bar = bar.bar - 1,
        }
    }
}
