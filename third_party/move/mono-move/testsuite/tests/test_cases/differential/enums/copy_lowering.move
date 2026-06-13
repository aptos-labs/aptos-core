// RUN: publish --print(micro-ops)
module 0x42::enum_copy_lowering {
    enum E has copy, drop { V { f: u64 } }
    struct Two has copy, drop { a: E, b: E }

    // Whole-enum copy: a byte copy of the 8-byte pointer (Move8) followed by a
    // DeepCopy of the single owned pointer at offset 0, so the copy is an
    // independent value rather than an alias of the source's heap object.
    fun enum_copy(): u64 {
        let a = E::V { f: 1 };
        let b = a;
        a.f + b.f
    }

    // Scalar copy: no owned heap pointers, so a bare byte copy (no DeepCopy) —
    // the hot path stays untouched.
    fun scalar_copy(): u64 {
        let a = 7u64;
        let b = a;
        a + b
    }

    // Multi-pointer aggregate copy: byte-copy the 16-byte struct, then DeepCopy
    // both embedded enum pointers (offsets 0 and 8).
    fun struct_copy(): u64 {
        let s = Two { a: E::V { f: 1 }, b: E::V { f: 2 } };
        let t = s;
        s.a.f + t.b.f
    }
}

// RUN: execute 0x42::enum_copy_lowering::enum_copy
// CHECK: results: 2

// RUN: execute 0x42::enum_copy_lowering::scalar_copy
// CHECK: results: 14

// RUN: execute 0x42::enum_copy_lowering::struct_copy
// CHECK: results: 3
