// RUN: publish
module 0x42::enum_ptr_at_offset {
    enum Inner has drop {
        Zero,
        Val { n: u64 },
    }

    // `inner` is an 8-byte enum heap pointer sitting at data offset 8 (after the
    // u64 `lead`), so the descriptor's GC pointer offset is non-zero.
    enum Outer has drop {
        Wrap { lead: u64, inner: Inner },
    }

    fun lead_plus_inner(a: u64, b: u64): u64 {
        let o = Outer::Wrap { lead: a, inner: Inner::Val { n: b } };
        match (o) {
            Outer::Wrap { lead, inner } => {
                let from_inner = match (inner) {
                    Inner::Zero => 0,
                    Inner::Val { n } => n,
                };
                lead + from_inner
            }
        }
    }
}

// RUN: execute 0x42::enum_ptr_at_offset::lead_plus_inner --args 40, 2
// CHECK: results: 42
