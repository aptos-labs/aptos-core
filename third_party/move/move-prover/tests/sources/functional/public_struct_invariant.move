// Tests that compiler-generated struct API wrappers (pack$S, unpack$S, borrow$S$f,
// borrow_mut$S$f, pack_variant$S$V, etc.) do NOT cause verification failures
// when a public struct or enum carries a data invariant.
module 0x42::public_struct_invariant {

    // -------------------------------------------------------------------------
    // Public struct with a data invariant
    // -------------------------------------------------------------------------

    public struct Counter has drop {
        value: u64,
    }

    spec Counter {
        invariant self.value > 0;
    }

    // The precondition ensures the invariant holds at the Pack site.
    public fun make(v: u64): Counter {
        Counter { value: v }
    }
    spec make {
        requires v > 0;
        ensures result.value == v;
    }

    // Mutable borrow: invariant must hold when the &mut param goes out of scope.
    public fun increment(c: &mut Counter) {
        c.value = c.value + 1;
    }

    // -------------------------------------------------------------------------
    // Public enum with a variant-guarded data invariant
    // -------------------------------------------------------------------------

    public enum Color has drop {
        Red { intensity: u64 },
        Green,
    }

    spec Color {
        invariant (self is Color::Red) ==> self.intensity > 0;
    }

    // Green carries no constrained field — always valid.
    public fun make_green(): Color {
        Color::Green
    }

    // Precondition ensures the intensity constraint holds.
    public fun make_red(i: u64): Color {
        Color::Red { intensity: i }
    }
    spec make_red {
        requires i > 0;
    }
}
