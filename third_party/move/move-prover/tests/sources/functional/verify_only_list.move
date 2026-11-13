// flag: --verify-only=verify_only_list::first
// flag: --verify-only=verify_only_list::second
// Repeating `--verify-only` verifies every named function, and only those:
// `third` has a false postcondition as well, but is not reported.
module 0x42::verify_only_list {
    fun first(): u64 { 1 }
    spec first {
        ensures result == 2; // error: `first` returns 1
    }

    fun second(): u64 { 2 }
    spec second {
        ensures result == 3; // error: `second` returns 2
    }

    fun third(): u64 { 3 }
    spec third {
        ensures result == 4;
    }
}
