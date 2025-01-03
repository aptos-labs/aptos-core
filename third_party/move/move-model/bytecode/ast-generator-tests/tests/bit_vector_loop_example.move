module std::m {
    use std::vector;

    /// The provided index is out of bounds
    const EINDEX: u64 = 0x20000;
    /// An invalid length of bitvector was given
    const ELENGTH: u64 = 0x20001;

    const WORD_SIZE: u64 = 1;
    /// The maximum allowed bitvector size
    const MAX_SIZE: u64 = 1024;


    struct BitVector has copy, drop, store {
        length: u64,
        bit_field: vector<bool>,
    }

    /// Set the bit at `bit_index` in the `self` regardless of its previous state.
    public fun set(self: &mut BitVector, bit_index: u64) {
        assert!(bit_index < vector::length(&self.bit_field), EINDEX);
        let x = vector::borrow_mut(&mut self.bit_field, bit_index);
        *x = true;
    }

    /// Unset the bit at `bit_index` in the `self` regardless of its previous state.
    public fun unset(self: &mut BitVector, bit_index: u64) {
        assert!(bit_index < vector::length(&self.bit_field), EINDEX);
        let x = vector::borrow_mut(&mut self.bit_field, bit_index);
        *x = false;
    }

    /// Shift the `self` left by `amount`. If `amount` is greater than the
    /// bitvector's length the bitvector will be zeroed out.
    public fun shift_left(self: &mut BitVector, amount: u64) {
        if (amount >= self.length) {
            vector::for_each_mut(&mut self.bit_field, |elem| {
                *elem = false;
            });
        } else {
            let i = amount;

            while (i < self.length) {
                if (is_index_set(self, i)) set(self, i - amount)
                else unset(self, i - amount);
                i = i + 1;
            };

            i = self.length - amount;

            while (i < self.length) {
                unset(self, i);
                i = i + 1;
            };
        }
    }


    /// Return the value of the bit at `bit_index` in the `self`. `true`
    /// represents "1" and `false` represents a 0
    public fun is_index_set(self: &BitVector, bit_index: u64): bool {
        assert!(bit_index < vector::length(&self.bit_field), EINDEX);
        *vector::borrow(&self.bit_field, bit_index)
    }
}
