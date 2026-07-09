// RUN: publish --print(micro-ops)
module 0x42::generic_enum_option {
    enum Maybe<T> has copy, drop {
        None,
        Some { value: T },
    }

    fun some<T>(v: T): Maybe<T> {
        Maybe::Some { value: v }
    }

    fun none<T>(): Maybe<T> {
        Maybe::None
    }

    fun unwrap_or<T: drop>(maybe: Maybe<T>, default: T): T {
        match (maybe) {
            Maybe::None => default,
            Maybe::Some { value } => value,
        }
    }

    fun run_some(v: u64): u64 {
        unwrap_or(some(v), 0)
    }

    fun run_none(default: u64): u64 {
        unwrap_or(none<u64>(), default)
    }

    fun run_bool(sel: u64): u64 {
        let maybe: Maybe<bool> = if (sel == 0) { none() } else { some(true) };
        if (unwrap_or(maybe, false)) { 1 } else { 0 }
    }

    // Copy semantics: both copies stay independently readable.
    fun copy_some(v: u64): u64 {
        let original = some(v);
        let duplicate = original;
        unwrap_or(original, 0) + unwrap_or(duplicate, 0)
    }
}

// RUN: execute 0x42::generic_enum_option::run_some --args 55
// CHECK: results: 55

// RUN: execute 0x42::generic_enum_option::run_none --args 77
// CHECK: results: 77

// RUN: execute 0x42::generic_enum_option::run_bool --args 0
// CHECK: results: 0

// RUN: execute 0x42::generic_enum_option::run_bool --args 1
// CHECK: results: 1

// RUN: execute 0x42::generic_enum_option::copy_some --args 21
// CHECK: results: 42
