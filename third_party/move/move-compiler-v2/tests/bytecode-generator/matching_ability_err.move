module 0xc0ffee::m {

    enum Inner {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }

    fun consume(self: Inner): bool {
        match (self) {
            Inner1{x: _} => {}
            Inner2{x: _, y:_ } => {}
        };
        true
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

    public fun condition_requires_copy(o: Outer): Outer {
        match (o) {
            One{i} if consume(i) => Outer::One{i},
            o => o
        }
    }
}
