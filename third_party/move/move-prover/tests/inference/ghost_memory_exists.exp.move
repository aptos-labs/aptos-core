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
        global counter: u64;
        ensures [inferred] 7 == counter ==> result == 7;
        aborts_if [inferred] false;
    }

    // Regression: propagating the opaque callee specification introduces an
    // internal existence guard for the spec variable's ghost memory.  Inferred
    // source must not expose the synthetic, unparseable `Ghost$counter` name.
    fun caller(): u64 {
        value()
    }
    spec caller(): u64 {
        pragma opaque = true;
        ensures [inferred] result == value();
        aborts_if [inferred] false;
    }

}
/*
Verification: exiting with compilation errors
error: duplicate declaration of `ghost_memory_exists::counter`
   ┌─ ghost_memory_exists.enriched.move:13:9
   │
13 │         global counter: u64;
   │         ^^^^^^^^^^^^^^^^^^^^
*/
