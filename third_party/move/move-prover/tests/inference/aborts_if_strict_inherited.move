// flag: --aborts-if-is-strict
// Strict aborts also reject partiality inherited from a callee whose own
// contract is partial.
module 0x42::aborts_if_strict_inherited {
    fun halve(x: u64): u64 {
        x / 2
    }
    spec halve {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures result == x / 2;
    }

    fun caller(x: u64): u64 {
        halve(x)
    }
}
