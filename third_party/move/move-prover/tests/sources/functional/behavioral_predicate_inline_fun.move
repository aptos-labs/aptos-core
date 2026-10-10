module 0x42::behavioral_predicate_inline_fun {
    inline fun increment(x: u64): u64 {
        x + 1
    }

    fun wrapper(x: u64): u64 {
        increment(x)
    }

    spec increment {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if x == MAX_U64;
        ensures result == x + 1;
    }

    spec wrapper {
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<increment>(x);
        ensures result == result_of<increment>(x);
    }
}
