module 0xc0ffee::m {
    use std::vector;

    public fun contains(features: &vector<u8>, feature: u64): bool {
        let bit_mask = 1 << (feature as u8);
        0 < vector::length(features) && (*vector::borrow(features, 0) & bit_mask) != 0
    }
}

