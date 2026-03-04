module 0xc0ffee::nested_match {

    enum Leaf {
        A,
        B,
    }

    enum Mid {
        M0{v: Leaf},
        M1{v: Leaf},
    }

    enum Top {
        T0{v: Mid},
        T1{v: Mid},
    }

    fun non_exhaustive_top(t: &Top): u8 {
        match (t) {
            Top::T0{v: _} => 0,
        }
    }

    fun non_exhaustive_nested(t: &Top): u8 {
        match (t) {
            Top::T0{v: Mid::M0{v: _}} => 0,
            Top::T1{v: _} => 1,
        }
    }

    fun unreachable_nested(t: &Top): u8 {
        match (t) {
            Top::T0{v: _} => 0,
            Top::T0{v: Mid::M0{v: _}} => 1,
            _ => 2,
        }
    }

    fun non_exhaustive_guard(t: &Top, cond: bool): u8 {
        match (t) {
            Top::T0{v: _} => 0,
            Top::T1{v: _} if cond => 1,
        }
    }

    fun tuple_enum_primitive(t: &Top, flag: bool): u8 {
        match ((t, flag)) {
            (Top::T0{v: _}, true) => 0,
            (Top::T1{v: _}, false) => 1,
        }
    }
}
