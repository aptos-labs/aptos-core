// RUN: publish
module 0x42::enums_nested {
    enum Inner has drop {
        Zero,
        Val { x: u64 },
    }

    // An enum whose variant field is itself another enum.
    enum Outer has drop {
        Wrap { inner: Inner, y: u64 },
    }

    fun nested_sum(x: u64, y: u64): u64 {
        let o = Outer::Wrap { inner: Inner::Val { x }, y };
        match (o) {
            Outer::Wrap { inner, y } => {
                let from_inner = match (inner) {
                    Inner::Zero => 0,
                    Inner::Val { x } => x,
                };
                from_inner + y
            }
        }
    }
    // `inner` is a heap pointer stored after the u64 `lead`, so the forced collection below
    // must trace a pointer that is not the variant's first field.
    enum OffsetOuter has drop {
        Wrap { lead: u64, inner: Inner },
    }
    fun lead_plus_inner(lead_value: u64, inner_value: u64): u64 {
        let outer = OffsetOuter::Wrap { lead: lead_value, inner: Inner::Val { x: inner_value } };
        0x0::test_utils::force_gc();
        match (outer) {
            OffsetOuter::Wrap { lead, inner } => {
                let from_inner = match (inner) {
                    Inner::Zero => 0,
                    Inner::Val { x: value } => value,
                };
                lead + from_inner
            }
        }
    }
}

// RUN: execute 0x42::enums_nested::nested_sum --args 25, 17
// CHECK: results: 42

// RUN: execute 0x42::enums_nested::lead_plus_inner --args 40, 2
// CHECK: results: 42
// CHECK-GC-COUNT: 1
