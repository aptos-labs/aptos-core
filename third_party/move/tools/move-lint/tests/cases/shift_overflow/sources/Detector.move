module NamedAddr::Detector {
    public fun func1(x: u64) {
        let _b = x << 24;
        let _b = x << 64; // <Issue:5>
        let _b = x << 65; // <Issue:5>
        let _b = x >> 66; // <Issue:5>
        let _u8 = (x as u8);
        let _u16 = (x as u16);
        let _u32 = (x as u32);
        let _u128 = (x as u128);
        let _u256 = (x as u256);

        let _b = _u8 << 8;
        let _b = _u16 << 16;
        let _b = _u32 << 32;
        let _b = _u128 << 128;
        let _b = _u256 << 128;
    }
}