// This file is created to verify the native function in the standard BCS module.
module 0x42::Arbitrary {

    fun arbitrary_u8_1(range: u8): u8 {
        if (range > 100) {
            abort 1
        } else {
            range
        }
    }

    spec fun arbitrary_u8_2(range: u8): u8 {
        if (range > 100) {
            abort (2 as u64)
        } else {
            range
        }
    }

    fun call_arbitrary_u8(range: u8): u8 {
        let x = range + 1;
        spec {
            // this cannot be proved because
            // call of arbitrary_u8_1 and arbitrary_u8_2 returns an arbitrary value
            // when range is not greater than 100
            assert arbitrary_u8_1(range) == arbitrary_u8_2(range);
            // this can be proved
            assert range <= 100 ==> arbitrary_u8_1(range) == arbitrary_u8_2(range);
        };
        x
    }


}
