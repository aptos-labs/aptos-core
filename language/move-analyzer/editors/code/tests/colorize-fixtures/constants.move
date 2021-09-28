module 0x1::M {
    const /* u64 maximum */ MAX_U64: u64 = 18446744073709551615u64;
    const MAX_U128 /* u128 maximum */ : u128 = 340282366920938463463374607431768211455u128;
    const TRUE: /* true constant */ bool = true;
    const FALSE: bool /* false constant */ = false;
    const ADDRESS: &address = /* address constant */ @0x42;
    const SHIFTED: u8 = 1 << 8 /* shifted constant */;
    const BYTES: vector<u8> = b"hello"; // bytes constant
    const KEY: vector<u8> = x"deadbeef";
    const CASTED: u64 = ((0: u128) as u64);
}
