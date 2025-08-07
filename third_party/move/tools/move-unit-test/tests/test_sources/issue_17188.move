module 0xc0ffee::m {
    use std::vector;

    #[test]
    public fun bad_cast1() {
        let x = 0;
        assert!(x == 0, 700);
        let big_number: u64 = 1000;
        let small_number: u8 = (big_number as u8);
        assert!(small_number == 255, 701);
    }

    #[test]
    public fun bad_cast2() {
        let x = 0;
        assert!(x == 0, 700);
        let big_number: u64 = 0;
        assert!(x == 0, 701);
        let small_number: u8 = (big_number as u8);
        while (big_number < 1000) {
            small_number = (big_number as u8);
            big_number = big_number + 1;
        };

        assert!(small_number == 255, 702);
    }

    #[test]
    public fun int_overflow1() {
        let x: u8 = 0;
        assert!(x == 0, 700);
        let y: u16 = 255;
        assert!(x == 0, 701);
        let z: u8 = (y as u8);
        assert!(x == 0, 702);
        let overflow: u8 = 1 + z;
        assert!(overflow <= 255, 703);
    }

    #[test]
    public fun int_overflow2() {
        let x: u8 = 0;
        assert!(x == 0, 700);
        let y: u16 = 255;
        y = y - 100;
        assert!(y <= 255, 701);
        let z: u8 = (y as u8);
        assert!(x == 0, 702);
        let overflow: u8 = z;
        while(overflow > 0) {
            overflow = overflow - 15;
            if (overflow > 15) {
                overflow = overflow - 1;
            };
        };
        assert!(overflow <= 255, 703);
    }

    #[test]
    public fun empty_vec() {
        let x: u8 = 0;
        assert!(x == 0, 700);
        vector::borrow(&vector::empty<u64>(), 1);
        assert!(x == 0, 700);
    }

    native fun foo();
    #[test]
    public fun non_existent_native() {
        let x: u8 = 0;
        assert!(x == 0, 700);
        foo();
        assert!(x == 0, 700);
    }
}
