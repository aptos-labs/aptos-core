//# publish
module 0xcafe::ConstantFailure {
    use std::vector;

    fun sum_u8(v: &vector<u8>): u8 {
        let sum: u8 = 0;
        vector::for_each_ref(v, |elt| sum = sum + *elt);
        sum
    }

    fun sum_u16(v: &vector<u16>): u16 {
        let sum: u16 = 0;
        vector::for_each_ref(v, |elt| sum = sum + *elt);
        sum
    }

    fun sum_u32(v: &vector<u32>): u32 {
        let sum: u32 = 0;
        vector::for_each_ref(v, |elt| sum = sum + *elt);
        sum
    }

    fun sum_u64(v: &vector<u64>): u64 {
        let sum: u64 = 0;
        vector::for_each_ref(v, |elt| sum = sum + *elt);
        sum
    }

    fun sum_u128(v: &vector<u128>): u128 {
        let sum: u128 = 0;
        vector::for_each_ref(v, |elt| sum = sum + *elt);
        sum
    }

    fun sum_u256(v: &vector<u256>): u256 {
        let sum: u256 = 0;
        vector::for_each_ref(v, |elt| sum = sum + *elt);
        sum
    }


    fun main() {
        // All of the following vector element value expressions should abort.
        //
        // Too bad we have no way to catch overflows and recover so we can
        // test a bunch of expressions in one program.
        //
        // We can at least check that Simplifier doesn't remove the
        // first one, anyway, and maybe scrutinize the bytecode
        // output.
        let fail_u8 = vector<u8>[
            1 << 8,
            0 >> 8,
            1 / 0,
            1 % 0,
            255 + 255,
            0 - 1,
            ((256: u64) as u8),
        ];
        let fail_u16 = vector<u16>[
            1 << 16,
            0 >> 16,
            1 / 0,
            1 % 0,
            65535 + 65535,
            0 - 1,
            ((65536: u64) as u16),
        ];
        let fail_u32 = vector<u32>[
            1 << 32,
            (1u8 << 8 as u32),
            ((1u16 << 16) as u32),
            0 >> 32,
            ((0u8 >> 8) as u32),
            ((0u16 >> 16) as u32),
            1 / 0,
            1 % 0,
            4294967295 + 4294967295,
            ((65535u16 + 65535) as u32),
            ((65535 + 65535u16) as u32),
            ((65535u16 + 65535u16) as u32),
            0 - 1,
            ((4294967296: u128) as u32),
        ];
        let fail_u64 = vector<u64>[
            1 << 64,
            ((1u32 << 32) as u64),
            0 >> 64,
            ((0u32 >> 32) as u64),
            1 / 0,
            1 % 0,
            18446744073709551615 + 18446744073709551615,
            ((255u8 + 255) as u64),
            ((255 + 255u8) as u64),
            ((255u8 + 255u8) as u64),
            ((4294967295u32 + 4294967295) as u64),
            ((4294967295 + 4294967295u32) as u64),
            ((4294967295u32 + 4294967295u32) as u64),
            0 - 1,
            ((340282366920938463463374607431768211450: u128) as u64),
        ];
        let fail_u128 = vector<u128>[
            1 << 128,
            ((1u64 << 64) as u128),
            0 >> 128,
            ((0u64 >> 64) as u128),
            1 / 0,
            1 % 0,
            340282366920938463463374607431768211450 + 340282366920938463463374607431768211450,
            ((18446744073709551615u64 + 18446744073709551615) as u128),
            ((18446744073709551615 + 18446744073709551615u64) as u128),
            ((18446744073709551615u64 + 18446744073709551615u64) as u128),
            0 - 1,
            ((340282366920938463463374607431768211456: u256) as u128),
        ];
        let fail_u256 = vector<u256>[
            ((1u128 << 128) as u256),
            ((0u128 >> 128) as u256),
            1 / 0,
            1 % 0,
            115792089237316195423570985008687907853269984665640564039457584007913129639935 + 115792089237316195423570985008687907853269984665640564039457584007913129639935,
            ((340282366920938463463374607431768211450u128 + 340282366920938463463374607431768211450) as u256),
            ((340282366920938463463374607431768211450 + 340282366920938463463374607431768211450u128) as u256),
            ((340282366920938463463374607431768211450u128 + 340282366920938463463374607431768211450u128) as u256),
            0 - 1,
        ];

        let sum8 = sum_u8(&fail_u8);
        let sum16 = sum_u16(&fail_u16);
        let sum32 = sum_u32(&fail_u32);
        let sum64 = sum_u64(&fail_u64);
        let sum128 = sum_u128(&fail_u128);
        let sum256 = sum_u256(&fail_u256);
        assert!(sum8 != 0, 1);
        assert!(sum16 != 0, 1);
        assert!(sum32 != 0, 1);
        assert!(sum64 != 0, 1);
        assert!(sum128 != 0, 1);
        assert!(sum256 != 0, 1);
    }
}

//# run 0xcafe::ConstantFailure::main
