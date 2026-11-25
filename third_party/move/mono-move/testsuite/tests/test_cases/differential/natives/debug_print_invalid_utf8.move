// `debug::native_print` of a String that is not UTF-8 is an invariant violation on MonoMove.
// The String comes from the friend-only `from_bcs::from_bytes`, declared public here;
// ordinary code cannot build it. Checked on MonoMove only: V1 panics on it in this harness.
// The control prints a one-byte UTF-8 String built the same way and succeeds on both VMs.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::debug {
    use std::string::String;

    native fun native_print(text: String);

    // Prints the String BCS-encoded as `bytes`, then returns its byte length.
    public fun print_encoded(bytes: vector<u8>): u64 {
        let text = 0x1::from_bcs::from_bytes<String>(bytes);
        native_print(text);
        text.length()
    }

    public fun print_valid(): u64 {
        print_encoded(x"0161")
    }

    public fun print_invalid(): u64 {
        print_encoded(x"01ff")
    }
}

// RUN: execute 0x1::debug::print_valid
// CHECK: results: 1

// RUN: execute 0x1::debug::print_invalid
// CHECK-V2: error: native function invariant violation: Every string must be UTF-8
