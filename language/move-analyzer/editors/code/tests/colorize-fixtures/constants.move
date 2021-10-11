module 0x1::M {
    const /* u64 maximum */ SUFFIX_U64: u64 = 18446744073709551615u64;
    const SUFFIX_U128 /* u128 maximum */ : u128 = 340282366920938463463374607431768211455u128;
    const HEX: u64 = 0x0Dead1Beef2;
    const TRUE: /* true constant */ bool = true;
    const FALSE: bool /* false constant */ = false;
    const ADDRESS_HEX: &address = /* address constant */ @0xAb34C;
    const ADDRESS_HEX_SUFFIX_U8: &address = @0xD23fu8 /* address constant */;
    const ADDRESS_DECIMAL: &address = @98760; // address constant
    const ADDRESS_DECIMAL_SUFFIX_U128: &address = @1248u128;
    const SHIFTED: u8 = 1 << 8;
    const BYTE_STRING: vector<u8> = b"he\nllo";
    const HEX_STRING: vector<u8> = x"0a1B2c3D4e5F6A7b8C9d0Ef";
    const HEX_STRING_INVALID: vector<u8> = x"00-ad\tZbeehxf";
    const CASTED: u64 = ((0: u128) as u64);
}
