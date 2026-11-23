// Recursion at the call-depth limits, in five shapes. V1 caps the call stack at 1024 frames
// and the operand stack at 1024 values; MonoMove caps its stack at 1 MiB. Depths checked:
//               both accept   only one accepts   both reject
//   direct_let  1023          26211 (MonoMove)   26212
//   closure     1023          21843 (MonoMove)   21844
//   cross       511           13105 (MonoMove)   13106
//   big         928           929 (V1)           1024
//   direct      1022          1023 (MonoMove)
// Depths between the two edges are where the VMs disagree and are checked per VM.

// RUN: publish
module 0x66::trampoline {
    public fun apply(func: |u64|u64, arg: u64): u64 { func(arg) }
}

module 0x66::depth {
    // Holds the pending `1` on V1's operand stack across each call.
    fun down(level: u64): u64 { if (level == 0) 0 else 1 + down(level - 1) }
    fun down_let(level: u64): u64 {
        if (level == 0) return 0;
        let result = down_let(level - 1);
        result + 1
    }
    fun via_closure(level: u64): u64 {
        if (level == 0) return 0;
        let recurse = |remaining| via_closure(remaining);
        let result = recurse(level - 1);
        result + 1
    }
    // Two frames per level: the trampoline and the closure target.
    fun via_other(level: u64): u64 {
        if (level == 0) return 0;
        let result = 0x66::trampoline::apply(|remaining| via_other(remaining), level - 1);
        result + 1
    }
    public fun direct(level: u64): u64 { down(level) }
    public fun direct_let(level: u64): u64 { down_let(level) }
    public fun closure(level: u64): u64 { via_closure(level) }
    public fun cross(level: u64): u64 { via_other(level) }

    // 32 u256 fields: 1 KiB of locals per call.
    struct Big has drop { f0: u256, f1: u256, f2: u256, f3: u256, f4: u256, f5: u256, f6: u256, f7: u256, f8: u256, f9: u256, f10: u256, f11: u256, f12: u256, f13: u256, f14: u256, f15: u256, f16: u256, f17: u256, f18: u256, f19: u256, f20: u256, f21: u256, f22: u256, f23: u256, f24: u256, f25: u256, f26: u256, f27: u256, f28: u256, f29: u256, f30: u256, f31: u256 }

    fun big_down(level: u64): u64 {
        let wide = (level as u256);
        let payload = Big { f0: wide, f1: wide, f2: wide, f3: wide, f4: wide, f5: wide, f6: wide, f7: wide, f8: wide, f9: wide, f10: wide, f11: wide, f12: wide, f13: wide, f14: wide, f15: wide, f16: wide, f17: wide, f18: wide, f19: wide, f20: wide, f21: wide, f22: wide, f23: wide, f24: wide, f25: wide, f26: wide, f27: wide, f28: wide, f29: wide, f30: wide, f31: wide };
        if (level == 0) return 0;
        let result = big_down(level - 1);
        result + ((payload.f0 - wide) as u64) + 1
    }
    public fun big(level: u64): u64 { big_down(level) }
}

// RUN: execute 0x66::depth::direct_let --args 1023
// CHECK: results: 1023

// RUN: execute 0x66::depth::direct_let --args 26212
// CHECK-V1-SUBSTR: CALL_STACK_OVERFLOW
// CHECK-V2-SUBSTR: stack overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::depth::direct_let --args 26211
// CHECK-V1-SUBSTR: CALL_STACK_OVERFLOW
// CHECK-V2: results: 26211

// RUN: execute 0x66::depth::direct --args 1022
// CHECK: results: 1022

// RUN: execute 0x66::depth::direct --args 1023
// CHECK-V1-SUBSTR: EXECUTION_STACK_OVERFLOW
// CHECK-V2: results: 1023

// RUN: execute 0x66::depth::closure --args 1023
// CHECK: results: 1023

// RUN: execute 0x66::depth::closure --args 21844
// CHECK-V1-SUBSTR: CALL_STACK_OVERFLOW
// CHECK-V2-SUBSTR: stack overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::depth::closure --args 21843
// CHECK-V1-SUBSTR: CALL_STACK_OVERFLOW
// CHECK-V2: results: 21843

// RUN: execute 0x66::depth::cross --args 511
// CHECK: results: 511

// RUN: execute 0x66::depth::cross --args 13106
// CHECK-V1-SUBSTR: CALL_STACK_OVERFLOW
// CHECK-V2-SUBSTR: stack overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::depth::cross --args 13105
// CHECK-V1-SUBSTR: CALL_STACK_OVERFLOW
// CHECK-V2: results: 13105

// RUN: execute 0x66::depth::big --args 928
// CHECK: results: 928

// RUN: execute 0x66::depth::big --args 929
// CHECK-V1: results: 929
// CHECK-V2: error: stack overflow

// RUN: execute 0x66::depth::big --args 1024
// CHECK-V1-SUBSTR: CALL_STACK_OVERFLOW
// CHECK-V2-SUBSTR: stack overflow
// CHECK-ERROR-PARITY
