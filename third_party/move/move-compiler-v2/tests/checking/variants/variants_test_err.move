module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue,
    }

    enum Shape {
        Quadrant{length: u64},
        Circle{radius: u64}

    }

    fun test1(c: Color): bool {
        (c is Red|Circle)
    }

    fun test2(c: Color): bool {
        (c is Red|Shape::Circle)
    }

    enum Generic<T> {
        Foo(T),
        Bar(u64)
    }

    fun test_generic<T>(x: &Generic<T>): bool {
        (x is Foo<u64>)
    }

    fun test_generic_qualified<T>(x: &Generic<T>): bool {
        (x is Foo<T>|Bar<u64>)
    }
}
