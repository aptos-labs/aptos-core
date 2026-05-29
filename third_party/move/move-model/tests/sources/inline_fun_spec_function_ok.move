// Function-level spec blocks are accepted on inline functions whose
// parameters do not have function type.
module 0x42::M {
    public inline fun add_one(x: u64): u64 {
        x + 1
    }
    spec add_one {
        aborts_if x == 0xFFFFFFFFFFFFFFFF;
        ensures result == x + 1;
    }

    public inline fun double(x: u64): u64 {
        x * 2
    }
    spec double {
        pragma opaque;
        aborts_if x > 0xFFFFFFFFFFFFFFFF / 2;
        ensures result == x * 2;
    }
}
