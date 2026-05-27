// Regression test for https://github.com/aptos-labs/aptos-core/issues/19872
//
// Reading a common field via projection on `&mut Enum` (where the enum has
// 2+ variants sharing that field name) used to cause the Move-to-Boogie
// translator to emit an extra `$Dereference` call for each subsequent
// variant arm in the synthesized match, breaking Boogie type-checking.

module 0x42::enum_match_repro {
    enum Foo has key {
        V1 { val: u64 },
        V2 { val: u64 },
        V3 { val: u64 },
    }

    public fun read_val(self: &mut Foo): u64 {
        self.val
    }

    spec read_val {
        aborts_if false;
    }

    public fun write_val(self: &mut Foo, v: u64) {
        self.val = v;
    }

    spec write_val {
        aborts_if false;
        ensures self.val == v;
    }
}
