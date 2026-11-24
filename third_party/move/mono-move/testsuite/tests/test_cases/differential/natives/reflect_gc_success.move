// A collection forced inside a successful `reflect::resolve`, at its `Result` allocation
// with the closure already allocated; the `_fill` step shows the heap is sized for that.
// The `_called` and `_held` steps run two more collections with the closure or the `Result`
// live, then call the closure, so a stale pointer would read overwritten memory.

// RUN: publish
module 0x42::target {
    public fun add(lhs: u64, rhs: u64): u64 {
        lhs + rhs
    }
}
module 0x42::main {
    use std::features;
    use std::reflect;
    use std::string::utf8;

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    // Allocates less than one churn round, so the preamble can end where the
    // closure still fits and the `Result` does not.
    fun pad(): u64 {
        let junk = vector[1u64];
        junk.length()
    }

    // Nothing allocates after the native.
    public fun resolve_ok(rounds: u64): bool {
        churn(rounds);
        let _ = pad();
        let module_name = utf8(b"target");
        let func_name = utf8(b"add");
        let result = reflect::resolve<|u64, u64|u64 has copy + drop>(@0x42, &module_name, &func_name);
        let ok = result.is_ok();
        let _add = result.unwrap();
        ok
    }

    public fun resolve_ok_fill(rounds: u64): (bool, u64) {
        churn(rounds);
        let _ = pad();
        let module_name = utf8(b"target");
        let func_name = utf8(b"add");
        let enabled = features::is_function_reflection_enabled();
        let first = vector[1u64, 2, 3];
        (enabled && module_name.length() + func_name.length() > 0, first.length())
    }

    public fun resolve_ok_called(rounds: u64, more: u64): (u64, u64) {
        churn(rounds);
        let _ = pad();
        let module_name = utf8(b"target");
        let func_name = utf8(b"add");
        let add = reflect::resolve<|u64, u64|u64 has copy + drop>(@0x42, &module_name, &func_name).unwrap();
        let first = add(2, 3);
        churn(more);
        (first, add(40, 2))
    }

    public fun resolve_ok_held(rounds: u64, more: u64): u64 {
        churn(rounds);
        let _ = pad();
        let module_name = utf8(b"target");
        let func_name = utf8(b"add");
        let result = reflect::resolve<|u64, u64|u64 has copy + drop>(@0x42, &module_name, &func_name);
        churn(more);
        let add = result.unwrap();
        add(40, 2)
    }
}

// RUN: execute 0x42::main::resolve_ok_fill --args 2720 --heap-size 32768
// CHECK: results: true, 3
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::resolve_ok --args 2720 --heap-size 32768
// CHECK: results: true
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::resolve_ok_called --args 2720, 6800 --heap-size 32768
// CHECK: results: 5, 42
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::resolve_ok_held --args 2720, 6800 --heap-size 32768
// CHECK: results: 42
// CHECK-GC-COUNT: 3
