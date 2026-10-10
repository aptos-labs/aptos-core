// Struct type arguments are invariant: bytecode assignability requires complete struct
// instantiations to be equal. These mismatches must fail during source type checking instead of
// producing invalid bytecode. The functions below cover call, return, pack, local-binding, and
// equality contexts.
module 0xc0ffee::m {
    struct S<T> has drop, store { v: T }

    fun takes(x: S<|u64|u64 has drop>): bool {
        let S { v: _ } = x;
        true
    }

    // Rejects widening in a call argument.
    public fun via_call(s: S<|u64|u64 has copy + drop>): bool {
        takes(s)
    }

    // Rejects widening in a return value.
    public fun via_return(s: S<|u64|u64 has copy + drop>): S<|u64|u64 has drop> {
        s
    }

    // Rejects widening while packing the outer struct.
    public fun via_pack(s: S<|u64|u64 has copy + drop>): S<S<|u64|u64 has drop>> {
        S { v: s }
    }

    // Rejects widening to an annotated local type.
    public fun via_let(s: S<|u64|u64 has copy + drop>): bool {
        let t: S<|u64|u64 has drop> = s;
        let S { v: _ } = t;
        true
    }

    // Rejects widening between equality operands.
    public fun via_eq(a: S<|u64|u64 has copy + drop>, b: S<|u64|u64 has drop>): bool {
        b == a
    }

    // Rejects equality widening through immutable references.
    public fun via_eq_ref(a: &S<|u64|u64 has copy + drop>, b: &S<|u64|u64 has drop>): bool {
        b == a
    }

    // Control case: identical instantiations remain accepted.
    public fun same_instantiation(a: S<|u64|u64 has drop>, b: S<|u64|u64 has drop>): bool {
        b == a
    }
}
