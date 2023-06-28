// Multiple return value similar to an example from the Move Book.

module 0x1::Math {
    public fun max(a: u8, b: u8): (u8, bool) {
        if (a > b) {
            (a, false)
        } else if (a < b) {
            (b, false)
        } else {
            (a, true)
        }
    }
}

script {
    use 0x1::Math;

    fun main()  {
        let (maxval, is_equal) = Math::max(99, 100);
        assert!(maxval == 100, 0xf00);
        assert!(!is_equal, 0xf01);

        let (maxval, is_equal) = Math::max(5, 0);
        assert!(maxval == 5, 0xf02);
        assert!(!is_equal, 0xf03);

        let (maxval, is_equal) = Math::max(123, 123);
        assert!(maxval == 123, 0xf04);
        assert!(is_equal, 0xf05);
    }
}
