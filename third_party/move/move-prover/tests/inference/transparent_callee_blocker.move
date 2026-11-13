// A transparent callee without a complete opaque contract blocks caller WP
// unless its body describes its behavior exactly: `string::length` does, the
// loop in `local_helper` does not. The dependency-scope variant of the
// diagnostic is covered by `vault.move`.
// flag: --verify-only=transparent_callee_blocker::caller
module 0x42::transparent_callee_blocker {
    use std::string;

    fun local_helper(x: u64): u64 {
        let i = 0;
        while (i < x) {
            i = i + 1;
        };
        i
    }

    fun caller(): u64 {
        let text = string::utf8(b"");
        local_helper(text.length())
    }
}
