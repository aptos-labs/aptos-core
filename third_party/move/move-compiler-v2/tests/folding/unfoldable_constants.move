address 0x42 {
module M {
    const SHL0: u8 = 1 << 8;
    const SHL1: u64 = 1 << 64;
    const SHL2: u128 = 1 << 128;
    const SHL3: u16 = 1 << 16;
    const SHL4: u32 = 1 << 32;

    const SHL0B: u32 = (1u8 << 8 as u32);
    const SHL1B: u128 = ((1u64 << 64) as u128);
    const SHL2B: u256 = ((1u128 << 128) as u256);
    const SHL3B: u32 = ((1u16 << 16) as u32);
    const SHL4B: u64 = ((1u32 << 32) as u64);

    const SHR0: u8 = 0 >> 8;
    const SHR1: u64 = 0 >> 64;
    const SHR2: u128 = 0 >> 128;
    const SHR3: u16 = 0 >> 16;
    const SHR4: u32 = 0 >> 32;

    const SHR0B: u32 = ((0u8 >> 8) as u32);
    const SHR1B: u128 = ((0u64 >> 64) as u128);
    const SHR2B: u256 = ((0u128 >> 128) as u256);
    const SHR3B: u32 = ((0u16 >> 16) as u32);
    const SHR4B: u64 = ((0u32 >> 32) as u64);

    const DIV0: u8 = 1 / 0;
    const DIV1: u64 = 1 / 0;
    const DIV2: u128 = 1 / 0;
    const DIV3: u16 = 1 / 0;
    const DIV4: u32 = 1 / 0;
    const DIV5: u256 = 1 / 0;

    const MOD0: u8 = 1 % 0;
    const MOD1: u64 = 1 % 0;
    const MOD2: u128 = 1 % 0;
    const MOD3: u16 = 1 % 0;
    const MOD4: u32 = 1 % 0;
    const MOD5: u256 = 1 % 0;

    const ADD0: u8 = 255 + 255;
    const ADD1: u64 = 18446744073709551615 + 18446744073709551615;
    const ADD2: u128 =
        340282366920938463463374607431768211450 + 340282366920938463463374607431768211450;
    const ADD3: u16 = 65535 + 65535;
    const ADD4: u32 = 4294967295 + 4294967295;
    const ADD5: u256 =
        115792089237316195423570985008687907853269984665640564039457584007913129639935 + 115792089237316195423570985008687907853269984665640564039457584007913129639935;

    const ADD0B: u64 = ((255u8 + 255) as u64);
    const ADD0C: u64 = ((255 + 255u8) as u64);
    const ADD0D: u64 = ((255u8 + 255u8) as u64);
    const ADD1B: u128 = ((18446744073709551615u64 + 18446744073709551615) as u128);
    const ADD1C: u128 = ((18446744073709551615 + 18446744073709551615u64) as u128);
    const ADD1D: u128 = ((18446744073709551615u64 + 18446744073709551615u64) as u128);
    const ADD2B: u256 =
        ((340282366920938463463374607431768211450u128 + 340282366920938463463374607431768211450) as u256);
    const ADD2C: u256 =
        ((340282366920938463463374607431768211450 + 340282366920938463463374607431768211450u128) as u256);
    const ADD2D: u256 =
        ((340282366920938463463374607431768211450u128 + 340282366920938463463374607431768211450u128) as u256);
    const ADD3B: u32 = ((65535u16 + 65535) as u32);
    const ADD3C: u32 = ((65535 + 65535u16) as u32);
    const ADD3D: u32 = ((65535u16 + 65535u16) as u32);
    const ADD4B: u64 = ((4294967295u32 + 4294967295) as u64);
    const ADD4C: u64 = ((4294967295 + 4294967295u32) as u64);
    const ADD4D: u64 = ((4294967295u32 + 4294967295u32) as u64);

    const SUB0: u8 = 0 - 1;
    const SUB1: u64 = 0 - 1;
    const SUB2: u128 = 0 - 1;
    const SUB3: u16 = 0 - 1;
    const SUB4: u32 = 0 - 1;
    const SUB5: u256 = 0 - 1;

    const CAST0: u8 = ((256: u64) as u8);
    const CAST1: u64 = ((340282366920938463463374607431768211450: u128) as u64);
    const CAST4: u128 = ((340282366920938463463374607431768211456: u256) as u128);
    const CAST2: u16 = ((65536: u64) as u16);
    const CAST3: u32 = ((4294967296: u128) as u32);
}
}
