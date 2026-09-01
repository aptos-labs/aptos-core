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
        aborts_if [inferred] false;
    }

}
/*
Verification: exiting with verification errors
error: post-condition does not hold
   ┌─ ghost_memory_exists.enriched.move:12:9
   │
12 │         ensures result == counter;
   │         ^^^^^^^^^^^^^^^^^^^^^^^^^^
   │
   =     at ghost_memory_exists.enriched.move:6: value
   =     at ghost_memory_exists.enriched.move:7: value
   =         result = <redacted>
   =     at ghost_memory_exists.enriched.move:13: value (spec)
   =     at ghost_memory_exists.enriched.move:12: value (spec)
*/
