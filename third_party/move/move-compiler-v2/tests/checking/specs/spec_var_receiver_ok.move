module 0x42::M {
    struct S has copy, drop {
        value: u64
    }
    fun get_value(self: &S): u64 {
        self.value
    }

    spec module {
        global last: u64;
    }

    fun record(s: &S) {
        spec { update last = s.get_value(); };
    }
    spec record {
        ensures last == s.value;
    }
}
