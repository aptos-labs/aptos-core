/// A recursive call below `&&`, `||` or `==>` knows the left operand on its
/// path, which proves the measure descends.
module 0x42::recursive_short_circuit {
    spec module {
        fun any_big(v: vector<u64>, end: u64): bool {
            end > 0 && (any_big(v, end - 1) || v[end - 1] > 10)
        }

        fun all_small(v: vector<u64>, end: u64): bool {
            end == 0 || (all_small(v, end - 1) && v[end - 1] <= 10)
        }

        fun implied(v: vector<u64>, end: u64): bool {
            end > 0 ==> (implied(v, end - 1) && v[end - 1] <= 10)
        }
    }

    fun id(v: vector<u64>): vector<u64> {
        v
    }
    spec id {
        aborts_if false;
        ensures !any_big(result, 0);
        ensures all_small(result, 0);
        ensures implied(result, 0);
        ensures len(result) > 0 && result[0] > 10 ==> any_big(result, 1);
        ensures len(result) > 0 && result[0] <= 10 ==> all_small(result, 1);
        ensures len(result) > 0 && result[0] > 10 ==> !implied(result, 1);
    }
}
