
//////////////////////////////////////////////////////////////////////
// Auto‑generated chain graph  –  8 modules.
// Path not selected
//////////////////////////////////////////////////////////////////////

module 0xABCD::C0 {
    use 0xABCD::C1;

    public entry fun call(_account: &signer, depth: u64) {
        let _ = op(depth);
    }

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            C1::op(depth - 1) + 1
        }
    }
}

module 0xABCD::C1 {
    use 0xABCD::C2;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            C2::op(depth - 1) + 1
        }
    }
}

module 0xABCD::C2 {
    use 0xABCD::C3;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            C3::op(depth - 1) + 1
        }
    }
}

module 0xABCD::C3 {
    use 0xABCD::C4;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            C4::op(depth - 1) + 1
        }
    }
}

module 0xABCD::C4 {
    use 0xABCD::C5;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            C5::op(depth - 1) + 1
        }
    }
}

module 0xABCD::C5 {
    use 0xABCD::C6;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            C6::op(depth - 1) + 1
        }
    }
}

module 0xABCD::C6 {
    use 0xABCD::C7;

    public fun op(depth: u64): u64 {
        if (depth == 0) { 0 } else {
            C7::op() + 1
        }
    }
}

module 0xABCD::C7 {
    public fun op(): u64 { 1 }
}
