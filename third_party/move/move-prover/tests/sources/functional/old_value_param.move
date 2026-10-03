// A loop that advances a value parameter: the invariant can only relate the
// parameter to where it started through `old`, which in an inline property is
// the value at function entry.
module 0x42::old_value_param {

    fun count_up(start: u64, end: u64): u64 {
        let n = 0;
        while (start < end) {
            start += 1;
            n += 1;
        } spec {
            invariant start == old(start) + n;
            invariant old(start) < end ==> start <= end;
            invariant old(start) >= end ==> n == 0;
        };
        n
    }
    spec count_up {
        aborts_if false;
        ensures start < end ==> result == end - start;
        ensures start >= end ==> result == 0;
    }

    fun count_up_wrong(start: u64, end: u64): u64 {
        let n = 0;
        while (start < end) {
            start += 1;
            n += 1;
        } spec {
            invariant start == old(start) + n + 1; // error: invariant does not hold on entry
        };
        n
    }
}
