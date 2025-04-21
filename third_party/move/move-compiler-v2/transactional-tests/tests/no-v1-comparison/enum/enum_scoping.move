//# publish
module 0x42::m {

    enum Inner has drop {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }

    struct Box has drop {
        x: u64
    }

    enum Outer has drop {
        None,
        One{i: Inner},
        Two{i: Inner, b: Box},
    }

    /// Check for enum scoping bug;
    /// result should be 3, not 4.
    public fun check_scoping(self: &Inner): u64 {
        let x = 3;
        {
            let x = 4;
            {
                match (self) {
                    Inner1{x: _} => true,
                    _ => false
                };
            };
        };
        x
    }

    fun t1_check_scoping(): u64 {
        let o = Outer::One{i: Inner::Inner1{x: 43}};
        match (o) {
            None => 0,
            One{i} => i.check_scoping(),
            Two{i, b} => 3,
        }
    }
}

//# run 0x42::m::t1_check_scoping
