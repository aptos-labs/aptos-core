// A call into `deep_low::wide` that both re-enters `deep_low` while a module lock is active
// and exceeds the call depth reports the reentrancy, through a closure call and through a
// direct call. At depth 1022 that call is past V1's call-depth limit, and its large locals do
// not fit in what remains of MonoMove's stack. Without the lock (`run_unlocked`) both VMs
// overflow instead.

// RUN: publish
module 0x66::deep_low {
    struct Quad has copy, drop { f0: u256, f1: u256, f2: u256, f3: u256 }
    struct Kilo has copy, drop { f0: Quad, f1: Quad, f2: Quad, f3: Quad, f4: Quad, f5: Quad, f6: Quad, f7: Quad }
    struct Block has copy, drop { f0: Kilo, f1: Kilo, f2: Kilo, f3: Kilo, f4: Kilo, f5: Kilo, f6: Kilo, f7: Kilo, f8: Kilo, f9: Kilo, f10: Kilo, f11: Kilo, f12: Kilo, f13: Kilo, f14: Kilo, f15: Kilo }
    struct Pair has drop { f0: Block, f1: Block }

    // Keeps this module active while `walk` runs; `wide` is passed as a function value.
    public fun run(walk: |u64, (|u64|u64 has drop), bool|u64, depth: u64, by_closure: bool): u64 {
        walk(depth, |seed| wide(seed), by_closure)
    }

    public fun wide(seed: u64): u64 {
        let val = (seed as u256);
        let quad = Quad { f0: val, f1: val, f2: val, f3: val };
        let kilo = Kilo { f0: quad, f1: quad, f2: quad, f3: quad, f4: quad, f5: quad, f6: quad, f7: quad };
        let block = Block { f0: kilo, f1: kilo, f2: kilo, f3: kilo, f4: kilo, f5: kilo, f6: kilo, f7: kilo, f8: kilo, f9: kilo, f10: kilo, f11: kilo, f12: kilo, f13: kilo, f14: kilo, f15: kilo };
        let pair = Pair { f0: block, f1: block };
        ((pair.f1.f15.f7.f3 + 1) as u64)
    }
}

module 0x66::deep_mid {
    use 0x66::deep_low;

    struct Quad has copy, drop { f0: u256, f1: u256, f2: u256, f3: u256 }
    struct Pad has drop { f0: Quad, f1: Quad, f2: Quad, f3: Quad, f4: Quad, f5: Quad }

    public fun descend(depth: u64, leaf: |u64|u64 has drop, by_closure: bool): u64 {
        let val = (depth as u256);
        let quad = Quad { f0: val, f1: val, f2: val, f3: val };
        let pad = Pad { f0: quad, f1: quad, f2: quad, f3: quad, f4: quad, f5: quad };
        if (depth == 0) {
            if (by_closure) return leaf(7);
            return deep_low::wide(7)
        };
        let rest = descend(depth - 1, leaf, by_closure);
        rest + ((pad.f5.f3 - val) as u64)
    }
}

module 0x66::deep_top {
    use 0x66::deep_low;
    use 0x66::deep_mid;

    #[module_lock]
    public fun run(depth: u64, by_closure: bool): u64 {
        deep_low::run(|level, leaf, flag| deep_mid::descend(level, leaf, flag), depth, by_closure)
    }

    public fun run_unlocked(depth: u64, by_closure: bool): u64 {
        deep_low::run(|level, leaf, flag| deep_mid::descend(level, leaf, flag), depth, by_closure)
    }
}

// Shallow: the lock alone decides.
// RUN: execute 0x66::deep_top::run_unlocked --args 3, true
// CHECK: results: 8

// RUN: execute 0x66::deep_top::run_unlocked --args 3, false
// CHECK: results: 8

// RUN: execute 0x66::deep_top::run --args 3, true
// CHECK-V1: error: VMError with status RUNTIME_DISPATCH_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000066, name: Identifier("deep_mid") } and message Reentrancy disallowed: reentering `0000000000000000000000000000000000000000000000000000000000000066::deep_low` via function `wide` (module lock is active)
// CHECK-V2: error: call: `0x66::deep_low::wide` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x66::deep_top::run --args 3, false
// CHECK-V1: error: VMError with status RUNTIME_DISPATCH_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000066, name: Identifier("deep_mid") } and message Reentrancy disallowed: reentering `0000000000000000000000000000000000000000000000000000000000000066::deep_low` via function `wide` (module lock is active)
// CHECK-V2: error: call: `0x66::deep_low::wide` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// Depth 1022 without the lock: both VMs overflow.
// RUN: execute 0x66::deep_top::run_unlocked --args 1022, true
// CHECK-V1-SUBSTR: status CALL_STACK_OVERFLOW
// CHECK-V2: error: stack overflow
// CHECK-ERROR-PARITY

// RUN: execute 0x66::deep_top::run_unlocked --args 1022, false
// CHECK-V1-SUBSTR: status CALL_STACK_OVERFLOW
// CHECK-V2: error: stack overflow
// CHECK-ERROR-PARITY

// Depth 1022 with the lock: the same calls report the reentrancy.
// RUN: execute 0x66::deep_top::run --args 1022, true
// CHECK-V1: error: VMError with status RUNTIME_DISPATCH_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000066, name: Identifier("deep_mid") } and message Reentrancy disallowed: reentering `0000000000000000000000000000000000000000000000000000000000000066::deep_low` via function `wide` (module lock is active)
// CHECK-V2: error: call: `0x66::deep_low::wide` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x66::deep_top::run --args 1022, false
// CHECK-V1: error: VMError with status RUNTIME_DISPATCH_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000066, name: Identifier("deep_mid") } and message Reentrancy disallowed: reentering `0000000000000000000000000000000000000000000000000000000000000066::deep_low` via function `wide` (module lock is active)
// CHECK-V2: error: call: `0x66::deep_low::wide` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY
