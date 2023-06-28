
// This is move-stdlib/sources/bit_vector.move until we build move-stdlib.

//module std::bit_vector {
module 0x10::bit_vector {
    use 0x1::vector;

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
            vector::push_back(&mut bit_field, false);
            counter = counter + 1;
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

    /// Set the bit at `bit_index` in the `bitvector` regardless of its previous state.
    public fun set(bitvector: &mut BitVector, bit_index: u64) {
        assert!(bit_index < vector::length(&bitvector.bit_field), EINDEX);
        let x = vector::borrow_mut(&mut bitvector.bit_field, bit_index);
        *x = true;
    }
    spec set {
        include SetAbortsIf;
        ensures bitvector.bit_field[bit_index];
    }
    spec schema SetAbortsIf {
        bitvector: BitVector;
        bit_index: u64;
        aborts_if bit_index >= length(bitvector) with EINDEX;
    }

    /// Unset the bit at `bit_index` in the `bitvector` regardless of its previous state.
    public fun unset(bitvector: &mut BitVector, bit_index: u64) {
        assert!(bit_index < vector::length(&bitvector.bit_field), EINDEX);
        let x = vector::borrow_mut(&mut bitvector.bit_field, bit_index);
        *x = false;
    }
    spec unset {
        include UnsetAbortsIf;
        ensures !bitvector.bit_field[bit_index];
    }
    spec schema UnsetAbortsIf {
        bitvector: BitVector;
        bit_index: u64;
        aborts_if bit_index >= length(bitvector) with EINDEX;
    }

    /// Shift the `bitvector` left by `amount`. If `amount` is greater than the
    /// bitvector's length the bitvector will be zeroed out.
    public fun shift_left(bitvector: &mut BitVector, amount: u64) {
        if (amount >= bitvector.length) {
           let len = vector::length(&bitvector.bit_field);
           let i = 0;
           while (i < len) {
               let elem = vector::borrow_mut(&mut bitvector.bit_field, i);
               *elem = false;
               i = i + 1;
           };
        } else {
            let i = amount;

            while (i < bitvector.length) {
                if (is_index_set(bitvector, i)) set(bitvector, i - amount)
                else unset(bitvector, i - amount);
                i = i + 1;
            };

            i = bitvector.length - amount;

            while (i < bitvector.length) {
                unset(bitvector, i);
                i = i + 1;
            };
        }
    }

    /// Return the value of the bit at `bit_index` in the `bitvector`. `true`
    /// represents "1" and `false` represents a 0
    public fun is_index_set(bitvector: &BitVector, bit_index: u64): bool {
        assert!(bit_index < vector::length(&bitvector.bit_field), EINDEX);
        *vector::borrow(&bitvector.bit_field, bit_index)
    }
    spec is_index_set {
        include IsIndexSetAbortsIf;
        ensures result == bitvector.bit_field[bit_index];
    }
    spec schema IsIndexSetAbortsIf {
        bitvector: BitVector;
        bit_index: u64;
        aborts_if bit_index >= length(bitvector) with EINDEX;
    }
    spec fun spec_is_index_set(bitvector: BitVector, bit_index: u64): bool {
        if (bit_index >= length(bitvector)) {
            false
        } else {
            bitvector.bit_field[bit_index]
        }
    }

    /// Return the length (number of usable bits) of this bitvector
    public fun length(bitvector: &BitVector): u64 {
        vector::length(&bitvector.bit_field)
    }

    /// Returns the length of the longest sequence of set bits starting at (and
    /// including) `start_index` in the `bitvector`. If there is no such
    /// sequence, then `0` is returned.
    public fun longest_set_sequence_starting_at(bitvector: &BitVector, start_index: u64): u64 {
        assert!(start_index < bitvector.length, EINDEX);
        let index = start_index;

        // Find the greatest index in the vector such that all indices less than it are set.
        while (index < bitvector.length) {
            if (!is_index_set(bitvector, index)) break;
            index = index + 1;
        };

        index - start_index
    }

    #[test_only]
    public fun word_size(): u64 {
        WORD_SIZE
    }
}

// This file is copied from move-stdlib/sources/vector.move
// until we are able to build move-stdlib.
//
//module std::vector {
module 0x1::vector {
    /// The index into the vector is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 0x20000;

    #[bytecode_instruction]
    /// Create an empty vector.
    native public fun empty<Element>(): vector<Element>;

    #[bytecode_instruction]
    /// Return the length of the vector.
    native public fun length<Element>(v: &vector<Element>): u64;

    #[bytecode_instruction]
    /// Acquire an immutable reference to the `i`th element of the vector `v`.
    /// Aborts if `i` is out of bounds.
    native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;

    #[bytecode_instruction]
    /// Add element `e` to the end of the vector `v`.
    native public fun push_back<Element>(v: &mut vector<Element>, e: Element);

    #[bytecode_instruction]
    /// Return a mutable reference to the `i`th element in the vector `v`.
    /// Aborts if `i` is out of bounds.
    native public fun borrow_mut<Element>(v: &mut vector<Element>, i: u64): &mut Element;

    #[bytecode_instruction]
    /// Pop an element from the end of vector `v`.
    /// Aborts if `v` is empty.
    native public fun pop_back<Element>(v: &mut vector<Element>): Element;

    #[bytecode_instruction]
    /// Destroy the vector `v`.
    /// Aborts if `v` is not empty.
    native public fun destroy_empty<Element>(v: vector<Element>);

    #[bytecode_instruction]
    /// Swaps the elements at the `i`th and `j`th indices in the vector `v`.
    /// Aborts if `i` or `j` is out of bounds.
    native public fun swap<Element>(v: &mut vector<Element>, i: u64, j: u64);

    /// Return an vector of size one containing element `e`.
    public fun singleton<Element>(e: Element): vector<Element> {
        let v = empty();
        push_back(&mut v, e);
        v
    }

    /// Reverses the order of the elements in the vector `v` in place.
    public fun reverse<Element>(v: &mut vector<Element>) {
        let len = length(v);
        if (len == 0) return ();

        let front_index = 0;
        let back_index = len -1;
        while (front_index < back_index) {
            swap(v, front_index, back_index);
            front_index = front_index + 1;
            back_index = back_index - 1;
        }
    }

    /// Pushes all of the elements of the `other` vector into the `lhs` vector.
    public fun append<Element>(lhs: &mut vector<Element>, other: vector<Element>) {
        reverse(&mut other);
        while (!is_empty(&other)) push_back(lhs, pop_back(&mut other));
        destroy_empty(other);
    }

    /// Return `true` if the vector `v` has no elements and `false` otherwise.
    public fun is_empty<Element>(v: &vector<Element>): bool {
        length(v) == 0
    }

    /// Return true if `e` is in the vector `v`.
    /// Otherwise, returns false.
    public fun contains<Element>(v: &vector<Element>, e: &Element): bool {
        let i = 0;
        let len = length(v);
        while (i < len) {
            if (borrow(v, i) == e) return true;
            i = i + 1;
        };
        false
    }

    /// Return `(true, i)` if `e` is in the vector `v` at index `i`.
    /// Otherwise, returns `(false, 0)`.
    public fun index_of<Element>(v: &vector<Element>, e: &Element): (bool, u64) {
        let i = 0;
        let len = length(v);
        while (i < len) {
            if (borrow(v, i) == e) return (true, i);
            i = i + 1;
        };
        (false, 0)
    }

    /// Remove the `i`th element of the vector `v`, shifting all subsequent elements.
    /// This is O(n) and preserves ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun remove<Element>(v: &mut vector<Element>, i: u64): Element {
        let len = length(v);
        // i out of bounds; abort
        if (i >= len) abort EINDEX_OUT_OF_BOUNDS;

        len = len - 1;
        while (i < len) swap(v, i, { i = i + 1; i });
        pop_back(v)
    }

    /// Insert `e` at position `i` in the vector `v`.
    /// If `i` is in bounds, this shifts the old `v[i]` and all subsequent elements to the right.
    /// If `i == length(v)`, this adds `e` to the end of the vector.
    /// This is O(n) and preserves ordering of elements in the vector.
    /// Aborts if `i > length(v)`
    public fun insert<Element>(v: &mut vector<Element>, e: Element, i: u64) {
        let len = length(v);
        // i too big abort
        if (i > len) abort EINDEX_OUT_OF_BOUNDS;

        push_back(v, e);
        while (i < len) {
            swap(v, i, len);
            i = i + 1
        }
    }

    /// Swap the `i`th element of the vector `v` with the last element and then pop the vector.
    /// This is O(1), but does not preserve ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun swap_remove<Element>(v: &mut vector<Element>, i: u64): Element {
        assert!(!is_empty(v), EINDEX_OUT_OF_BOUNDS);
        let last_idx = length(v) - 1;
        swap(v, i, last_idx);
        pop_back(v)
    }
}

module 0x10::bit_vector_tests {
    use 0x10::bit_vector;

    fun test_bitvector_set_unset_of_size(k: u64) {
        let bitvector = bit_vector::new(k);
        let index = 0;
        while (index < k) {
            bit_vector::set(&mut bitvector, index);
            assert!(bit_vector::is_index_set(&bitvector, index), 0);
            index = index + 1;
            let index_to_right = index;
            while (index_to_right < k) {
                assert!(!bit_vector::is_index_set(&bitvector, index_to_right), 1);
                index_to_right = index_to_right + 1;
            };
        };
        // now go back down unsetting
        index = 0;

        while (index < k) {
            bit_vector::unset(&mut bitvector, index);
            assert!(!bit_vector::is_index_set(&bitvector, index), 0);
            index = index + 1;
            let index_to_right = index;
            while (index_to_right < k) {
                assert!(bit_vector::is_index_set(&bitvector, index_to_right), 1);
                index_to_right = index_to_right + 1;
            };
        };
    }

    public fun test_set_bit_and_index_basic() {
        test_bitvector_set_unset_of_size(8)
    }

    public fun test_set_bit_and_index_odd_size() {
        test_bitvector_set_unset_of_size(33)
    }

    public fun longest_sequence_no_set_zero_index() {
        let bitvector = bit_vector::new(100);
        assert!(bit_vector::longest_set_sequence_starting_at(&bitvector, 0) == 0, 0);
    }

    public fun longest_sequence_one_set_zero_index() {
        let bitvector = bit_vector::new(100);
        bit_vector::set(&mut bitvector, 1);
        assert!(bit_vector::longest_set_sequence_starting_at(&bitvector, 0) == 0, 0);
    }

    public fun longest_sequence_no_set_nonzero_index() {
        let bitvector = bit_vector::new(100);
        assert!(bit_vector::longest_set_sequence_starting_at(&bitvector, 51) == 0, 0);
    }

    public fun longest_sequence_two_set_nonzero_index() {
        let bitvector = bit_vector::new(100);
        bit_vector::set(&mut bitvector, 50);
        bit_vector::set(&mut bitvector, 52);
        assert!(bit_vector::longest_set_sequence_starting_at(&bitvector, 51) == 0, 0);
    }

    public fun longest_sequence_with_break() {
        let bitvector = bit_vector::new(100);
        let i = 0;
        while (i < 20) {
            bit_vector::set(&mut bitvector, i);
            i = i + 1;
        };
        // create a break in the run
        i = i + 1;
        while (i < 100) {
            bit_vector::set(&mut bitvector, i);
            i = i + 1;
        };
        assert!(bit_vector::longest_set_sequence_starting_at(&bitvector, 0) == 20, 0);
        assert!(bit_vector::longest_set_sequence_starting_at(&bitvector, 20) == 0, 0);
        assert!(bit_vector::longest_set_sequence_starting_at(&bitvector, 21) == 100 - 21, 0);
    }

    public fun test_shift_left() {
        let bitlen = 53;
        let bitvector = bit_vector::new(bitlen);

        let i = 0;
        while (i < bitlen) {
            bit_vector::set(&mut bitvector, i);
            i = i + 1;
        };

        i = bitlen - 1;
        while (i > 0) {
            assert!(bit_vector::is_index_set(&bitvector, i), 0);
            bit_vector::shift_left(&mut bitvector, 1);
            assert!(!bit_vector::is_index_set(&bitvector,  i), 1);
            i = i - 1;
        };
    }

    public fun test_shift_left_specific_amount() {
        let bitlen = 300;
        let shift_amount = 133;
        let bitvector = bit_vector::new(bitlen);

        bit_vector::set(&mut bitvector, 201);
        assert!(bit_vector::is_index_set(&bitvector, 201), 0);

        bit_vector::shift_left(&mut bitvector, shift_amount);
        assert!(bit_vector::is_index_set(&bitvector, 201 - shift_amount), 1);
        assert!(!bit_vector::is_index_set(&bitvector, 201), 2);

        // Make sure this shift clears all the bits
        bit_vector::shift_left(&mut bitvector, bitlen  - 1);

        let i = 0;
        while (i < bitlen) {
            assert!(!bit_vector::is_index_set(&bitvector, i), 3);
            i = i + 1;
        }
    }

    public fun test_shift_left_specific_amount_to_unset_bit() {
        let bitlen = 50;
        let chosen_index = 24;
        let shift_amount = 3;
        let bitvector = bit_vector::new(bitlen);

        let i = 0;

        while (i < bitlen) {
            bit_vector::set(&mut bitvector, i);
            i = i + 1;
        };

        bit_vector::unset(&mut bitvector, chosen_index);
        assert!(!bit_vector::is_index_set(&bitvector, chosen_index), 0);

        bit_vector::shift_left(&mut bitvector, shift_amount);

        i = 0;

        while (i < bitlen) {
            // only chosen_index - shift_amount and the remaining bits should be BitVector::unset
            if ((i == chosen_index - shift_amount) || (i >= bitlen - shift_amount)) {
                assert!(!bit_vector::is_index_set(&bitvector, i), 1);
            } else {
                assert!(bit_vector::is_index_set(&bitvector, i), 2);
            };
            i = i + 1;
        }
    }

    public fun shift_left_at_size() {
        let bitlen = 133;
        let bitvector = bit_vector::new(bitlen);

        let i = 0;
        while (i < bitlen) {
            bit_vector::set(&mut bitvector, i);
            i = i + 1;
        };

        bit_vector::shift_left(&mut bitvector, bitlen - 1);
        i = bitlen - 1;
        while (i > 0) {
            assert!(!bit_vector::is_index_set(&bitvector,  i), 1);
            i = i - 1;
        };
    }

    public fun shift_left_more_than_size() {
        let bitlen = 133;
        let bitvector = bit_vector::new(bitlen);
        bit_vector::shift_left(&mut bitvector, bitlen);
    }

    public fun single_bit_bitvector() {
        let bitvector = bit_vector::new(1);
        assert!(bit_vector::length(&bitvector) == 1, 0);
    }
}

script {
    use 0x10::bit_vector_tests as BT;

    fun main() {
        BT::test_set_bit_and_index_basic();
        BT::test_set_bit_and_index_odd_size();
        BT::longest_sequence_no_set_zero_index();
        BT::longest_sequence_one_set_zero_index();
        BT::longest_sequence_no_set_nonzero_index();
        BT::longest_sequence_two_set_nonzero_index();
        BT::longest_sequence_with_break();
        BT::test_shift_left();
        BT::test_shift_left_specific_amount();
        BT::test_shift_left_specific_amount_to_unset_bit();
        BT::shift_left_at_size();
        BT::shift_left_more_than_size();
        BT::single_bit_bitvector();
    }
}
