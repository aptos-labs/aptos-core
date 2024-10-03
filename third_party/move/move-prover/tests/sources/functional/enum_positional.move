module 0x815::m {

    enum Positional has drop {
        A(u8),
        B(u8),
    }

    spec Positional {
        invariant self.0 > 20;
    }

    fun test_positional_incorrect(): u8 {
        let x = Positional::A(42);
        spec {
           assert x.0 == 42;
        };
        match (&mut x) {
            Positional::A(y) => *y = 3, // aborts
            B(y) => *y = 50
        };
        20
    }

    fun test_positional_correct(): u8 {
        let x = Positional::A(42);
        spec {
            assert x.0 == 42;
        };
        let z = &mut x;
        match (z) {
            Positional::A(y) => *y = 21,
            B(y) => *y = 2
        };
        20
    }



}
