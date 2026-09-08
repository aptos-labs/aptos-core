// A transparent callee without a complete opaque contract blocks caller WP.
// The diagnostic distinguishes a callee in the editable target module from a
// dependency outside that scope.
// flag: --verify-only=transparent_callee_blocker::caller
module 0x42::transparent_callee_blocker {
    use std::string;

    fun local_helper(x: u64): u64 {
        x + 1
    }

    fun caller(): u64 {
        let text = string::utf8(b"");
        local_helper(text.length())
    }
}
