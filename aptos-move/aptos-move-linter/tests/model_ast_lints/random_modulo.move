module 0xc0ffee::m {
    use aptos_framework::randomness::{
        u8_integer,
        u8_range,
        u16_integer,
        u16_range,
        u32_integer,
        u32_range,
        u64_integer,
        u64_range,
        u128_integer,
        u128_range,
        u256_integer,
        u256_range,
    };

    public fun test1_warn(): u256 {
        let ret = (u8_integer() % 251) as u256;
        ret += (u16_integer() % 65521) as u256;
        ret += (u32_integer() % 2147483647) as u256;
        ret += (u64_integer() % 2305843009213693951) as u256;
        ret += (u128_integer() % 170141183460469231731687303715884105727) as u256;
        ret += u256_integer() % 77194726158210796949047323339125271902179989777093709359638389338608753093290;
        ret
    }

    public fun test2_warn(): u256 {
        let ret = (u8_integer() % (u8_range(0, 251) ^ 0xAA)) as u256;
        ret += (u16_integer() % (u16_range(0, 65521) ^ 0xAAAA)) as u256;
        ret += (u32_integer() % (u32_range(0, 2147483647) ^ 0xAAAAAAAA)) as u256;
        ret += (u64_integer() % (u64_range(0, 2305843009213693951) ^ 0xAAAAAAAA)) as u256;
        ret += (u128_integer() % (u128_range(0, 170141183460469231731687303715884105727) ^ 0xAAAAAAAAAAAAAAAA)) as u256;
        ret += u256_integer() % (u256_range(0, 77194726158210796949047323339125271902179989777093709359638389338608753093290) ^ 0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA);
        ret
    }
}
