module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue(),
    }

    fun t0(): bool {
        let c = Color::Red; // no error expected
        c.red == 1
    }
    fun t1(): bool {
        let c : Color = Red; // no error expected
        c.red == 1
    }

    fun t2(): bool {
        let c : Color = Blue; // no error expected
        c.red == 1
    }

    fun t3(): bool {
        let c : Color = Blue(); // no error expected
        c.red == 1
    }

    fun t4(c: &Color) {
        match (c) { Red => abort 1, Blue => abort 2 } // no error
    }
}
