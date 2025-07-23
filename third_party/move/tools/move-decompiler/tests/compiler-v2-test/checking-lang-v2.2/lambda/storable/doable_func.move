module 0x42::mod1 {
    package fun triple(x: u64) : u64 {
        x * 3
    }
}

module 0x42::mod2 {
    friend 0x42::test;
    friend fun double(x: u64): u64 {
        x * 2
    }
}

module 0x42::mod3 {
    public fun multiply(x: u64, y: u64): u64 {
        x * y
    }
}

module 0x42::mod4 {
    public fun alt_multiply(x: u64, y: u64): u64 {
        x * y
    }
}

module 0x42::test {
    use 0x42::mod1;
    use 0x42::mod2;
    use 0x42::mod3;
    use 0x42::mod4::alt_multiply;
    fun multiply3(x: u64, y: u64, z: u64): u64 {
        x * y * z
    }

    // compute ((key + 2) * x) in different ways
    fun choose_function1(key: u64, x: u64): u64 {
        let f =
            if (key == 0) {
                mod2::double
            } else if (key == 1) {
                mod1::triple
            } else if (key == 2) {
                |x| mod3::multiply(4, x)
            } else if (key == 3) {
                let x = 5;
                |y| alt_multiply(x, y)
            } else if (key == 4) {
                let x = 6;
                |y| mod3::multiply(x, y)
            } else if (key == 5) {
                |x| multiply3(3, 2, x)
            } else if (key == 6) {
                |x| mod3::multiply(7, x)
            } else if (key == 7) {
                |x| { multiply3(4, 2, x) }
            } else if (key == 8) {
                |x| multiply3(3, 3, x)
            } else if (key == 9) {
                let x = 2;
                let y = 5;
                |z| multiply3(x + 1, y, z)
            } else if (key == 10) {
                let z = 11;
                |x| alt_multiply(z, x)
            } else if (key == 11) {
                let g = |x, y| mod3::multiply(x, y);
                |x| g(11, x)
            } else if (key == 12) {
                let h = |x| mod3::multiply(12, x);
                |x| { h(x) }
            } else if (key == 14) {
                let i = |x| multiply3(2, 2, x);
                |z| i(z)
            } else {
                let i = |x, y| { let q = y - 1; 0x42::mod3::multiply(x, q + 1)  };
                |x| i(15, x)
            };
        f(x)
    }

    fun add_mul(x: u64, y: u64, z: u64): u64 {
        z * (x + y)
    }

    public fun test_functions() {
        let x = 3;

        for (i in 0..15) {
            let y = choose_function1(i, 3);
            assert!(y == (i + 2) * x, i);
        }
    }
}
