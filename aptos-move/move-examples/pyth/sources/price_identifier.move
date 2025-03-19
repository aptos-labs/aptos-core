module pyth::price_identifier {
    struct PriceIdentifier has copy, drop, store {
        bytes: vector<u8>
    }

    public fun from_byte_vec(_bytes: vector<u8>): PriceIdentifier {
        abort 0
    }
}
