module std::bit_vector {
    use std::vector;

    /// The provided index is out of bounds
    const EINDEX: u64 = 0x20000;
    /// An invalid length of bitvector was given
    const ELENGTH: u64 = 0x20001;

    const WORD_SIZE: u64 = 1;
    /// The maximum allowed bitvector size
    const MAX_SIZE: u64 = 1024;

    spec BitVector {
        invariant length == len(bit_field);
    }

    struct BitVector has copy, drop, store {
        length: u64,
        bit_field: vector<bool>,
    }

    public fun new(length: u64): BitVector {
        assert!(length > 0, ELENGTH);
        assert!(length < MAX_SIZE, ELENGTH);
        let counter = 0;
        let bit_field = vector::empty();
        while ({spec {
            invariant counter <= length;
            invariant len(bit_field) == counter;
        };
            (counter < length)}) {
            bit_field.push_back(false);
            counter += 1;
        };
        spec {
            assert counter == length;
            assert len(bit_field) == length;
        };

        BitVector {
            length,
            bit_field,
        }
    }
    spec new {
        include NewAbortsIf;
        ensures result.length == length;
        ensures len(result.bit_field) == length;
    }
    spec schema NewAbortsIf {
        length: u64;
        aborts_if length <= 0 with ELENGTH;
        aborts_if length >= MAX_SIZE with ELENGTH;
    }

    /// Set the bit at `bit_index` in the `self` regardless of its previous state.
    public fun set(self: &mut BitVector, bit_index: u64) {
        assert!(bit_index < self.bit_field.length(), EINDEX);
        self.bit_field[bit_index] = true;
    }
    spec set {
        include SetAbortsIf;
        ensures self.bit_field[bit_index];
    }
    spec schema SetAbortsIf {
        self: BitVector;
        bit_index: u64;
        aborts_if bit_index >= self.length() with EINDEX;
    }

    /// Unset the bit at `bit_index` in the `self` regardless of its previous state.
    public fun unset(self: &mut BitVector, bit_index: u64) {
        assert!(bit_index < self.bit_field.length(), EINDEX);
        self.bit_field[bit_index] = false;
    }
    spec unset {
        include UnsetAbortsIf;
        ensures !self.bit_field[bit_index];
    }
    spec schema UnsetAbortsIf {
        self: BitVector;
        bit_index: u64;
        aborts_if bit_index >= self.length() with EINDEX;
    }

    /// Shift the `self` left by `amount`. If `amount` is greater than the
    /// bitvector's length the bitvector will be zeroed out.
    public fun shift_left(self: &mut BitVector, amount: u64) {
        if (amount >= self.length) {
            self.bit_field.for_each_mut(|elem| {
                *elem = false;
            });
        } else {
            let i = amount;

            while (i < self.length) {
                if (self.is_index_set(i)) self.set(i - amount)
                else self.unset(i - amount);
                i += 1;
            };

            i = self.length - amount;

            while (i < self.length) {
                self.unset(i);
                i += 1;
            };
        }
    }
    spec shift_left {
        // TODO: set to false because data invariant cannot be proved with inline function. Will remove it once inline is supported
        pragma verify = false;
    }

    /// Return the value of the bit at `bit_index` in the `self`. `true`
    /// represents "1" and `false` represents a 0
    public fun is_index_set(self: &BitVector, bit_index: u64): bool {
        assert!(bit_index < self.bit_field.length(), EINDEX);
        self.bit_field[bit_index]
    }
    spec is_index_set {
        include IsIndexSetAbortsIf;
        ensures result == self.bit_field[bit_index];
    }
    spec schema IsIndexSetAbortsIf {
        self: BitVector;
        bit_index: u64;
        aborts_if bit_index >= self.length() with EINDEX;
    }
    spec fun spec_is_index_set(self: BitVector, bit_index: u64): bool {
        if (bit_index >= self.length()) {
            false
        } else {
            self.bit_field[bit_index]
        }
    }

    /// Return the length (number of usable bits) of this bitvector
    public fun length(self: &BitVector): u64 {
        self.bit_field.length()
    }

    /// Returns the length of the longest sequence of set bits starting at (and
    /// including) `start_index` in the `bitvector`. If there is no such
    /// sequence, then `0` is returned.
    public fun longest_set_sequence_starting_at(self: &BitVector, start_index: u64): u64 {
        assert!(start_index < self.length, EINDEX);
        let index = start_index;

        // Find the greatest index in the vector such that all indices less than it are set.
        while ({
            spec {
                invariant index >= start_index;
                invariant index == start_index || self.is_index_set(index - 1);
                invariant index == start_index || index - 1 < len(self.bit_field);
                invariant forall j in start_index..index: self.is_index_set(j);
                invariant forall j in start_index..index: j < len(self.bit_field);
            };
            index < self.length
        }) {
            if (!self.is_index_set(index)) break;
            index += 1;
        };

        index - start_index
    }

    spec longest_set_sequence_starting_at(self: &BitVector, start_index: u64): u64 {
        aborts_if start_index >= self.length;
        ensures forall i in start_index..result: self.is_index_set(i);
    }

    #[test_only]
    public fun word_size(): u64 {
        WORD_SIZE
    }

    #[verify_only]
    public fun shift_left_for_verification_only(self: &mut BitVector, amount: u64) {
        if (amount >= self.length) {
            let len = self.bit_field.length();
            let i = 0;
            while ({
                spec {
                    invariant len == self.length;
                    invariant forall k in 0..i: !self.bit_field[k];
                    invariant forall k in i..self.length: self.bit_field[k] == old(self).bit_field[k];
                };
                i < len
            }) {
                let elem = self.bit_field.borrow_mut(i);
                *elem = false;
                i += 1;
            };
        } else {
            let i = amount;

            while ({
                spec {
                    invariant i >= amount;
                    invariant self.length == old(self).length;
                    invariant forall j in amount..i: old(self).bit_field[j] == self.bit_field[j - amount];
                    invariant forall j in (i-amount)..self.length : old(self).bit_field[j] == self.bit_field[j];
                    invariant forall k in 0..i-amount: self.bit_field[k] == old(self).bit_field[k + amount];
                };
                i < self.length
            }) {
                if (self.is_index_set(i)) self.set(i - amount)
                else self.unset(i - amount);
                i += 1;
            };


            i = self.length - amount;

            while ({
                spec {
                    invariant forall j in self.length - amount..i: !self.bit_field[j];
                    invariant forall k in 0..self.length - amount: self.bit_field[k] == old(self).bit_field[k + amount];
                    invariant i >= self.length - amount;
                };
                i < self.length
            }) {
                self.unset(i);
                i += 1;
            }
        }
    }
    spec shift_left_for_verification_only {
        aborts_if false;
        ensures amount >= self.length ==> (forall k in 0..self.length: !self.bit_field[k]);
        ensures amount < self.length ==>
            (forall i in self.length - amount..self.length: !self.bit_field[i]);
        ensures amount < self.length ==>
            (forall i in 0..self.length - amount: self.bit_field[i] == old(self).bit_field[i + amount]);
    }
}
