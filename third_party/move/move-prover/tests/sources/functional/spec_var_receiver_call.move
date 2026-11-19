// Receiver-style calls in the `update` of a global spec variable and of a ghost field.
module 0x42::spec_var_receiver_call {
    struct S has copy, drop { f: u8 }
    struct G has copy, drop { s: S }

    fun get_f(self: &S): u8 { self.f }

    spec module {
        global last_f: u8;
    }

    spec G {
        ghost g: u8 = 0;
    }

    fun record(s: &S) {
        spec { update last_f = s.get_f(); };
    }
    spec record {
        ensures last_f == s.f;
    }

    fun record_incorrect(s: &S) {
        spec { update last_f = s.get_f(); };
    }
    spec record_incorrect {
        ensures last_f == s.f + 1;
    }

    fun record_ghost(x: &mut G) {
        spec { update x.g = x.s.get_f(); };
    }
    spec record_ghost {
        ensures x.g == x.s.f;
    }

    fun record_ghost_incorrect(x: &mut G) {
        spec { update x.g = x.s.get_f(); };
    }
    spec record_ghost_incorrect {
        ensures x.g == x.s.f + 1;
    }
}
