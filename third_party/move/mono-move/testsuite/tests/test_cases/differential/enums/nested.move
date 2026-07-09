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
}

// RUN: execute 0x42::enums_nested::nested_sum --args 25, 17
// CHECK: results: 42
