module 0x815::m {

    enum Color {
        RGB{red: u64, green: u64, blue: u64},
        Red,
        Blue(u64),
    }

    fun take(_c: Color){}

    fun t1(): bool {
        let c : Color = Red{}; // no error expected
        c.red == 1
    }

    fun t2() {
        take(Red{}) // errors expected because of bottom-up type inference
    }

    fun t3(): Color {
        let c = Red{}; // error expected
        c
    }

    fun t4(): Color {
        let c = Color{}; // error expected
        c
    }

    fun t5(): Color {
        let c : Color = Blue(0); // no error expected
        c
    }
}
