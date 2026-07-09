// RUN: publish --print(micro-ops)
module 0x42::enum_copy_lowering {
    enum E has copy, drop { V { f: u64 } }
    struct Two has copy, drop { a: E, b: E }

    fun enum_copy(): u64 {
        let a = E::V { f: 1 };
        let b = a;
        a.f + b.f
    }

    fun scalar_copy(): u64 {
        let a = 7u64;
        let b = a;
        a + b
    }

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
