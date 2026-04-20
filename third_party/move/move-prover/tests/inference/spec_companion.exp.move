// Test that companion .spec.move files don't corrupt output_unified output.
// Bug 8b: output_unified matched spec blocks by their source position without
// checking file_id. Spec blocks from a companion .spec.move file have byte
// offsets into the companion file, not the main .move file — inserting them
// at those offsets into the .move content garbles the output.
//
// The fix: output_unified checks `info.loc.file_id() == file_id` before using
// a spec block's position, skipping companion-file blocks entirely.
//
// `increment` and `get` have their specs in spec_companion.spec.move.
// `caller` has no spec and exercises inference alongside the companion specs.
module 0x42::spec_companion {
    struct Counter has copy, drop { value: u64 }

    // Specs live in spec_companion.spec.move with `pragma inference = none`.
    fun increment(self: &mut Counter) {
        self.value = self.value + 1;
    }

    fun get(self: &Counter): u64 {
        self.value
    }

    // No spec — inference runs here.
    // `increment` has a &mut param → behavior predicate.
    // After increment, self is the post-state; reading self.value gives result.
    fun caller(self: &mut Counter): u64 {
        increment(self);
        self.value
    }
    spec caller(self: &mut Counter): u64 {
        pragma opaque = true;
        ensures [inferred] result == result_of<increment>(old(self)).value;
        ensures [inferred] self == result_of<increment>(old(self));
        ensures [inferred] ensures_of<increment>(old(self), result_of<increment>(old(self)));
        aborts_if [inferred] aborts_of<increment>(self);
    }

}
/*
Verification: Succeeded.
*/
