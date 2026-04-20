//# publish
module 0xc0ffee::m {
    enum Color has drop {
        RGB(u8, u8, u8),
        Grayscale(u8),
        Named,
    }

    fun classify(c: Color, x: u64): u64 {
        match ((c, x)) {
            (Color::RGB(0..25, 28, 55..66), 88..99) => 1,
            (Color::RGB(_, _, _), _) => 2,
            (Color::Grayscale(128..=255), 0..50) => 3,
            (Color::Grayscale(_), _) => 4,
            (Color::Named, _) => 5,
        }
    }

    // Arm 1: all conditions match
    public fun test_rgb_all_match(): u64 {
        classify(Color::RGB(10, 28, 60), 90)
    }

    // Arm 2: RGB but first field out of range
    public fun test_rgb_first_out(): u64 {
        classify(Color::RGB(30, 28, 60), 90)
    }

    // Arm 2: RGB but second field mismatch
    public fun test_rgb_second_mismatch(): u64 {
        classify(Color::RGB(10, 29, 60), 90)
    }

    // Arm 2: RGB but tuple range mismatch
    public fun test_rgb_tuple_out(): u64 {
        classify(Color::RGB(10, 28, 60), 100)
    }

    // Arm 3: grayscale high + low tuple
    public fun test_gray_high_low(): u64 {
        classify(Color::Grayscale(200), 25)
    }

    // Arm 4: grayscale high but tuple out of range
    public fun test_gray_high_tuple_out(): u64 {
        classify(Color::Grayscale(200), 50)
    }

    // Arm 4: grayscale low value
    public fun test_gray_low(): u64 {
        classify(Color::Grayscale(100), 25)
    }

    // Arm 5: named
    public fun test_named(): u64 {
        classify(Color::Named, 0)
    }
}

//# run 0xc0ffee::m::test_rgb_all_match

//# run 0xc0ffee::m::test_rgb_first_out

//# run 0xc0ffee::m::test_rgb_second_mismatch

//# run 0xc0ffee::m::test_rgb_tuple_out

//# run 0xc0ffee::m::test_gray_high_low

//# run 0xc0ffee::m::test_gray_high_tuple_out

//# run 0xc0ffee::m::test_gray_low

//# run 0xc0ffee::m::test_named
