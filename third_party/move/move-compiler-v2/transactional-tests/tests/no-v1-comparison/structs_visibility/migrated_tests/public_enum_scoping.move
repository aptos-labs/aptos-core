//# publish
module 0x42::m {

    public enum Inner has drop {
        Inner1{ x: u64 }
        Inner2{ x: u64, y: u64 }
    }

    struct Box has drop {
        x: u64
    }

    public enum Outer has drop {
        None,
        One{i: Inner},
        Two{i: Inner, b: Box},
    }

}

//# publish
module 0x42::test_m {
    use 0x42::m::Inner;
    use 0x42::m::Outer;

    /// Check for enum scoping bug;
    /// result should be 3, not 4.
    public fun check_scoping(i: &Inner): u64 {
        let x = 3;
        {
            let x = 4;
            {
                match (i) {
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
            One{i} => check_scoping(&i),
            Two{i, b} => 3,
        }
    }
}

//# run 0x42::test_m::t1_check_scoping
