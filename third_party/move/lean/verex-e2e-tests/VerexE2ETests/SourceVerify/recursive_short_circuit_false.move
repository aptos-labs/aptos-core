/// A left operand knows nothing of the right one: the recursive call below
/// runs before its guard.
module 0x42::recursive_short_circuit_false {
    spec module {
        fun unguarded(v: vector<u64>, end: u64): bool {
            unguarded(v, end - 1) && end > 0
        }
    }

    fun id(v: vector<u64>): vector<u64> {
        v
    }
    spec id {
        ensures !unguarded(result, 0); // error: no measure of `unguarded` decreases
    }
}
