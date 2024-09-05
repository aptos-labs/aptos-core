module 0x42::TestInvariants {
    spec module {
        pragma verify = true;
    }

    struct PositionalStruct(u64, u64) has key, copy, drop;

    spec PositionalStruct {
        invariant self.0 > 10;
        invariant self.1 < 10;
    }

    fun pack_positional_struct_correct(): PositionalStruct {
        PositionalStruct(12, 8)
    }

    fun pack_positional_struct_incorrect(): PositionalStruct {
        PositionalStruct(8, 12)
    }

}
