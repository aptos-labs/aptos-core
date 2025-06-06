
//////////////////////////////////////////////////////////////////////
// Auto‑generated chain graph  –  8 modules.
// Path not selected
//////////////////////////////////////////////////////////////////////

module 0xABCD::chain_8_0 {
    use 0xABCD::chain_8_1;

    public entry fun call(_account: &signer, depth: u64) {
        let _ = op(depth);
    }

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            chain_8_1::op(depth - 1) + 1
        }
    }
}

module 0xABCD::chain_8_1 {
    use 0xABCD::chain_8_2;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            chain_8_2::op(depth - 1) + 1
        }
    }
}

module 0xABCD::chain_8_2 {
    use 0xABCD::chain_8_3;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            chain_8_3::op(depth - 1) + 1
        }
    }
}

module 0xABCD::chain_8_3 {
    use 0xABCD::chain_8_4;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            chain_8_4::op(depth - 1) + 1
        }
    }
}

module 0xABCD::chain_8_4 {
    use 0xABCD::chain_8_5;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            chain_8_5::op(depth - 1) + 1
        }
    }
}

module 0xABCD::chain_8_5 {
    use 0xABCD::chain_8_6;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            chain_8_6::op(depth - 1) + 1
        }
    }
}

module 0xABCD::chain_8_6 {
    use 0xABCD::chain_8_7;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            chain_8_7::op() + 1
        }
    }
}

module 0xABCD::chain_8_7 {
    public fun op(): u64 { 1 }
}
