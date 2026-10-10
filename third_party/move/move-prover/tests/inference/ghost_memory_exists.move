module 0x42::ghost_memory_exists {
    spec module {
        global counter: u64;
    }

    fun value(): u64 {
        7
    }

    spec value {
        pragma opaque;
        ensures result == counter;
    }

    // Regression: propagating the opaque callee specification introduces an
    // internal existence guard for the spec variable's ghost memory.  Inferred
    // source must not expose the synthetic, unparseable `Ghost$counter` name.
    fun caller(): u64 {
        value()
    }
}
