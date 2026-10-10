// RUN: publish
module 0x66::located {
    fun deep(x: u64, y: u64): u64 { x / y }

    fun middle(x: u64, y: u64): u64 { deep(x, y) }

    public fun three_frames_deep(x: u64, y: u64): u64 { middle(x, y) }

    public fun after_loop(n: u64, d: u64): u64 {
        let acc = 0;
        let i = 0;
        while (i < n) {
            acc = acc + i;
            i = i + 1;
        };
        acc / d
    }

    fun read_at(v: &vector<u64>, i: u64): u64 { v[i] }

    public fun vector_error_one_frame_down(i: u64): u64 {
        let v = vector[1u64, 2, 3];
        read_at(&v, i)
    }
}

// Both VMs attribute a nested error to the innermost function.
// RUN: execute 0x66::located::three_frames_deep --args 1, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-ERROR-PARITY

// The offset identifies the original faulting bytecode after a loop, not the
// lowered micro-op position.
// RUN: execute 0x66::located::after_loop --args 3, 0
// CHECK-V1-SUBSTR: ARITHMETIC_ERROR
// CHECK-ERROR-PARITY

// The vector error checks both its sub-status and faulting definition index.
// RUN: execute 0x66::located::vector_error_one_frame_down --args 5
// CHECK-V1-SUBSTR: VECTOR_OPERATION_ERROR
// CHECK-ERROR-PARITY
