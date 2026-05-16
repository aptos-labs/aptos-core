// Companion spec file for spec_companion.move.
// These specs live in a separate file to exercise Bug 8b: output_unified must
// check file_id before using a spec block's byte position, otherwise companion
// file positions corrupt the .move source being unified.
spec 0x42::spec_companion {
    spec increment(self: &mut Counter) {
        pragma opaque = true;
        pragma inference = none;
        aborts_if self.value == MAX_U64;
        ensures self.value == old(self.value) + 1;
    }

    spec get(self: &Counter): u64 {
        pragma opaque = true;
        pragma inference = none;
        aborts_if false;
        ensures result == self.value;
    }
}
