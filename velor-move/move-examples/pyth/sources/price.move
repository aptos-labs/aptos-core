module pyth::price {
    use pyth::i64::I64;

    struct Price has copy, drop, store {
        price: I64,
        conf: u64,
        expo: I64,
        timestamp: u64
    }

    public fun new(_price: I64, _conf: u64, _expo: I64, _timestamp: u64): Price {
        abort 0
    }

    public fun get_price(_p: &Price): I64 {
        abort 0
    }

    public fun get_conf(_p: &Price): u64 {
        abort 0
    }

    public fun get_expo(_p: &Price): I64 {
        abort 0
    }

    public fun get_timestamp(_p: &Price): u64 {
        abort 0
    }
}
