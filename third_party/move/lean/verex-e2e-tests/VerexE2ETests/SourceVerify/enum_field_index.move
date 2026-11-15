/// An element of a vector field of an enum: the bound check reads the field
/// of whichever variant declares it.
module 0x42::enum_field_index {
    enum Chunks has drop {
        V1 { p: vector<u64>, r: vector<u64> },
        V2 { p: vector<u64> },
    }

    fun set(x: &mut u64, value: u64) {
        *x = value;
    }
    spec set {
        aborts_if false;
        ensures x == value;
    }

    fun set_p(self: &mut Chunks, i: u64) {
        set(&mut self.p[i], 7);
    }
    spec set_p {
        aborts_if i >= len(self.p);
        ensures self.p[i] == 7;
        ensures len(self.p) == len(old(self).p);
    }

    fun read_p(self: &Chunks, i: u64): u64 {
        self.p[i]
    }
    spec read_p {
        aborts_if i >= len(self.p);
        ensures result == self.p[i];
    }

    /// Only one variant declares `r`: another aborts at the selection.
    fun read_r(self: &Chunks, i: u64): u64 {
        self.r[i]
    }
    spec read_r {
        aborts_if self is V2;
        aborts_if (self is V1) && i >= len(self.r);
        ensures result == self.r[i];
    }
}
