module 0xc0ffee::m {

    enum Inner {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }

    struct Box has drop {
        x: u64
    }

    enum Outer {
        None,
        One{i: Inner},
        Two{i: Inner, b: Box},
    }

    public fun matched_value_not_consumed(o: Outer) {
        match (o) {
            One{i: _} => {}
            _ => {}
        }
    }

    // The guard checker permits comparisons as reads. Guard evaluation binds
    // `b` by reference, so comparing its value requires `Box` to have `copy`.
    // This test must fail the copy-ability check.
    public fun condition_compares_payload(o: Outer, other: Box): Outer {
        match (o) {
            Two{i, b} if (b == other) => Outer::Two{i, b},
            o => o
        }
    }
}
