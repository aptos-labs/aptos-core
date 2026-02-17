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

    // Exhaustiveness

    public fun non_exhaustive(o: &Outer) {
        match (o) {
            None => {}
            One{i: _} => {}
        }
    }

    public fun non_exhaustive_because_of_cond(o: &Outer) {
        match (o) {
            None => {}
            One{i: _} => {}
            Two{i: _, b} if b.x > 0 => {}
        }
    }

    public fun non_exhaustive_because_of_nested(o: &Outer) {
        match (o) {
            None => {}
            One{i: Inner1{x: _}} => {}
            Two{i: _, b: _} => {}
        }
    }

    public fun exhaustive_via_merge(o: &Outer) {
        match (o) {
            None => {}
            One{i: Inner1{x: _}} => {}
            One{i: Inner2{x: _, y: _}} => {}
            Two{i: _, b: _} => {}
        }
    }
    public fun non_exhaustive_tuple(i: &Inner) {
        match ((i, i)) {
            (Inner1{x: _}, _) => {}
        }
    }

    public fun exhaustive_tuple(i: &Inner) {
        match ((i, i)) {
            (Inner1{x: _}, _) => {}
            (Inner2{x: _, y: _}, _) => {}
        }
    }

    public fun non_exhaustive_tuple2(i: &Inner) {
        match ((i, i)) {
            (Inner1{x: _}, _) => {}
            (_, Inner2{x: _, y: _}) => {}
        }
    }

    // Reachability

    public fun unreachable(o: &Outer) {
         match (o) {
             None => {}
             One{i: _} => {}
             Two{i: _, b: _} => {}
             _ => {}
         }
    }

    public fun unreachable_via_repeated_pattern(o: &Outer) {
         match (o) {
             None => {}
             One{i: _} => {}
             One{i: _} => {}
             _ => {}
         }
    }

    public fun unreachable_via_overlaying_pattern(o: &Outer) {
         match (o) {
             None => {}
             One{i: Inner1{x:_}} => {}
             One{i: _} => {}
             One{i: Inner1{x:_}} => {}
             _ => {}
         }
    }
}
