module 0x42::ApplySchemaTestOnly {
    struct S has store { v: u64 }

    public fun bump(self: &mut S) {
        self.v = self.v + 1;
    }

    #[test]
    fun test_binds_no_self() {}

    #[test_only]
    fun helper_binds_no_self() {}

    spec schema Preserves {
        self: S;
        ensures old(self.v) <= self.v;
    }

    spec module {
        apply Preserves to *;
    }
}
