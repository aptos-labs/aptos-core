module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
    }

    fun test(c: Color): bool {
        c is Red|RGB
    }

    fun test_qualified(c: Color): bool {
        c is Color::Red|RGB
    }

    fun test_fully_qualified(c: Color): bool {
        c is 0x815::m::Color::Red
    }

    enum Generic<T> {
        Foo(T),
        Bar(u64)
    }

    fun test_generic<T>(x: &Generic<T>): bool {
        x is Foo<T>
    }

    fun test_generic_qualified<T>(x: &Generic<T>): bool {
        x is Generic::Foo<T>
    }
}
