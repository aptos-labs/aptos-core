// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x42::generic_call {
    fun identity<T>(x: T): T {
        x
    }

    fun call_u64(): u64 {
        identity<u64>(7)
    }

    fun call_bool(): bool {
        identity<bool>(true)
    }
}

// RUN: execute 0x42::generic_call::call_u64
// CHECK: results: 7

// RUN: execute 0x42::generic_call::call_bool
// CHECK: results: true
