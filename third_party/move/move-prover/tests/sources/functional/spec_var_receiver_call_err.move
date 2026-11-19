// Receiver-style calls in a global spec variable initializer and a ghost field initializer
// resolve to the Move function, which these positions reject like the form `get_f(s)`. The Lean
// backend stops at the unsupported global initializer; given only the ghost initializer, it
// accepts the call and verifies `track`.
module 0x42::spec_var_receiver_call_err {
    struct S has copy, drop { f: u8 }
    struct Tracked has copy, drop { s: S }

    fun get_f(self: &S): u8 { self.f }

    spec module {
        global initial: u8 = S { f: 3 }.get_f();
    }

    fun initial_value(): u8 { 3 }
    spec initial_value {
        ensures result == initial;
    }

    spec Tracked {
        ghost g: u8 = s.get_f();
        invariant g == s.f;
    }

    fun track(s: S): Tracked { Tracked { s } }
}
