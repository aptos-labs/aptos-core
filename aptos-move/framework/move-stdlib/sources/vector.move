/// A variable-sized container that can hold any type. Indexing is 0-based, and
/// vectors are growable. This module has many native functions.
/// Verification of modules that use this one uses model functions that are implemented
/// directly in Boogie. The specification language has built-in functions operations such
/// as `singleton_vector`. There are some helper functions defined here for specifications in other
/// modules as well.
///
/// >Note: We did not verify most of the
/// Move functions here because many have loops, requiring loop invariants to prove, and
/// the return on investment didn't seem worth it for these simple functions.
module std::vector {
    use std::mem;

    /// The index into the vector is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 0x20000;

    /// The index into the vector is out of bounds
    const EINVALID_RANGE: u64 = 0x20001;

    /// The length of the vectors are not equal.
    const EVECTORS_LENGTH_MISMATCH: u64 = 0x20002;

    /// The step provided in `range` is invalid, must be greater than zero.
    const EINVALID_STEP: u64 = 0x20003;

    /// The range in `slice` is invalid.
    const EINVALID_SLICE_RANGE: u64 = 0x20004;

    /// Whether to utilize native vector::move_range
    /// Vector module cannot call features module, due to cyclic dependency,
    /// so this is a constant.
    const USE_MOVE_RANGE: bool = true;

    #[bytecode_instruction]
    /// Create an empty vector.
    native public fun empty<Element>(): vector<Element>;

    #[bytecode_instruction]
    /// Return the length of the vector.
    native public fun length<Element>(self: &vector<Element>): u64;

    #[bytecode_instruction]
    /// Acquire an immutable reference to the `i`th element of the vector `self`.
    /// Aborts if `i` is out of bounds.
    native public fun borrow<Element>(self: &vector<Element>, i: u64): &Element;

    #[bytecode_instruction]
    /// Add element `e` to the end of the vector `self`.
    native public fun push_back<Element>(self: &mut vector<Element>, e: Element);

    #[bytecode_instruction]
    /// Return a mutable reference to the `i`th element in the vector `self`.
    /// Aborts if `i` is out of bounds.
    native public fun borrow_mut<Element>(self: &mut vector<Element>, i: u64): &mut Element;

    #[bytecode_instruction]
    /// Pop an element from the end of vector `self`.
    /// Aborts if `self` is empty.
    native public fun pop_back<Element>(self: &mut vector<Element>): Element;

    #[bytecode_instruction]
    /// Destroy the vector `self`.
    /// Aborts if `self` is not empty.
    native public fun destroy_empty<Element>(self: vector<Element>);

    #[bytecode_instruction]
    /// Swaps the elements at the `i`th and `j`th indices in the vector `self`.
    /// Aborts if `i` or `j` is out of bounds.
    native public fun swap<Element>(self: &mut vector<Element>, i: u64, j: u64);

    /// Moves range of elements `[removal_position, removal_position + length)` from vector `from`,
    /// to vector `to`, inserting them starting at the `insert_position`.
    /// In the `from` vector, elements after the selected range are moved left to fill the hole
    /// (i.e. range is removed, while the order of the rest of the elements is kept)
    /// In the `to` vector, elements after the `insert_position` are moved to the right to make
    /// space for new elements (i.e. range is inserted, while the order of the rest of the
    ///  elements is kept).
    /// Move prevents from having two mutable references to the same value, so `from` and `to`
    /// vectors are always distinct.
    native public fun move_range<T>(
        from: &mut vector<T>,
        removal_position: u64,
        length: u64,
        to: &mut vector<T>,
        insert_position: u64
    );

    /// Return an vector of size one containing element `e`.
    public inline fun singleton<Element>(e: Element): vector<Element> {
        let v = empty();
        v.push_back(e);
        v
    }

    /// Reverses the order of the elements in the vector `self` in place.
    public fun reverse<Element>(self: &mut vector<Element>) {
        let len = self.length();
        self.reverse_slice(0, len);
    }

    spec reverse {
        pragma intrinsic = true;
    }

    /// Reverses the order of the elements [left, right) in the vector `self` in place.
    public fun reverse_slice<Element>(self: &mut vector<Element>, left: u64, right: u64) {
        assert!(left <= right, EINVALID_RANGE);
        if (left == right) return;
        right -= 1;
        while (left < right) {
            self.swap(left, right);
            left += 1;
            right -= 1;
        }
    }
    spec reverse_slice {
        pragma intrinsic = true;
    }

    /// Pushes all of the elements of the `other` vector into the `self` vector.
    public fun append<Element>(self: &mut vector<Element>, other: vector<Element>) {
        if (USE_MOVE_RANGE) {
            let self_length = self.length();
            let other_length = other.length();
            move_range(&mut other, 0, other_length, self, self_length);
            other.destroy_empty();
        } else {
            other.reverse();
            self.reverse_append(other);
        }
    }
    spec append {
        pragma intrinsic = true;
    }

    /// Pushes all of the elements of the `other` vector into the `self` vector.
    public fun reverse_append<Element>(self: &mut vector<Element>, other: vector<Element>) {
        let len = other.length();
        while (len > 0) {
            self.push_back(other.pop_back());
            len -= 1;
        };
        other.destroy_empty();
    }
    spec reverse_append {
        pragma intrinsic = true;
    }

    /// Splits (trims) the collection into two at the given index.
    /// Returns a newly allocated vector containing the elements in the range [new_len, len).
    /// After the call, the original vector will be left containing the elements [0, new_len)
    /// with its previous capacity unchanged.
    /// In many languages this is also called `split_off`.
    public fun trim<Element>(self: &mut vector<Element>, new_len: u64): vector<Element> {
        let len = self.length();
        assert!(new_len <= len, EINDEX_OUT_OF_BOUNDS);

        let other = empty();
        if (USE_MOVE_RANGE) {
            move_range(self, new_len, len - new_len, &mut other, 0);
        } else {
            while (len > new_len) {
                other.push_back(self.pop_back());
                len -= 1;
            };
            other.reverse();
        };

        other
    }
    spec trim {
        pragma intrinsic = true;
    }

    /// Trim a vector to a smaller size, returning the evicted elements in reverse order
    public fun trim_reverse<Element>(self: &mut vector<Element>, new_len: u64): vector<Element> {
        let len = self.length();
        assert!(new_len <= len, EINDEX_OUT_OF_BOUNDS);
        let result = empty();
        while (new_len < len) {
            result.push_back(self.pop_back());
            len -= 1;
        };
        result
    }
    spec trim_reverse {
        pragma intrinsic = true;
    }


    /// Return `true` if the vector `self` has no elements and `false` otherwise.
    public inline fun is_empty<Element>(self: &vector<Element>): bool {
        self.length() == 0
    }

    /// Return true if `e` is in the vector `self`.
    public fun contains<Element>(self: &vector<Element>, e: &Element): bool {
        let i = 0;
        let len = self.length();
        while (i < len) {
            if (self.borrow(i) == e) return true;
            i += 1;
        };
        false
    }
    spec contains {
        pragma intrinsic = true;
    }

    /// Return `(true, i)` if `e` is in the vector `self` at index `i`.
    /// Otherwise, returns `(false, 0)`.
    public fun index_of<Element>(self: &vector<Element>, e: &Element): (bool, u64) {
        let i = 0;
        let len = self.length();
        while (i < len) {
            if (self.borrow(i) == e) return (true, i);
            i += 1;
        };
        (false, 0)
    }
    spec index_of {
        pragma intrinsic = true;
    }

    /// Return `(true, i)` if there's an element that matches the predicate. If there are multiple elements that match
    /// the predicate, only the index of the first one is returned.
    /// Otherwise, returns `(false, 0)`.
    public inline fun find<Element>(self: &vector<Element>, f: |&Element|bool): (bool, u64) {
        let find = false;
        let found_index = 0;
        let i = 0;
        let len = self.length();
        while ({
            spec {
                invariant i <= len;
                invariant forall j: num where j >= 0 && j < i: !f(self[j]);
                invariant find ==> i < len && f(self[i]);
            };
            i < len
            }) {
            // Cannot call return in an inline function so we need to resort to break here.
            if (f(self.borrow(i))) {
                find = true;
                found_index = i;
                break
            };
            i += 1;
        };
        spec {
            assert !find <==> (forall j: num where j >= 0 && j < len: !f(self[j]));
        };
        (find, found_index)
    }

    /// Insert a new element at position 0 <= i <= length, using O(length - i) time.
    /// Aborts if out of bounds.
    public fun insert<Element>(self: &mut vector<Element>, i: u64, e: Element) {
        let len = self.length();
        assert!(i <= len, EINDEX_OUT_OF_BOUNDS);

        if (USE_MOVE_RANGE) {
            if (i + 2 >= len) {
                // When we are close to the end, it is cheaper to not create
                // a temporary vector, and swap directly
                self.push_back(e);
                while (i < len) {
                    self.swap(i, len);
                    i += 1;
                };
            } else {
                let other = singleton(e);
                move_range(&mut other, 0, 1, self, i);
                other.destroy_empty();
            }
        } else {
            self.push_back(e);
            while (i < len) {
                self.swap(i, len);
                i += 1;
            };
        };
    }
    spec insert {
        pragma intrinsic = true;
    }

    /// Remove the `i`th element of the vector `self`, shifting all subsequent elements.
    /// This is O(n) and preserves ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun remove<Element>(self: &mut vector<Element>, i: u64): Element {
        let len = self.length();
        // i out of bounds; abort
        if (i >= len) abort EINDEX_OUT_OF_BOUNDS;

        if (USE_MOVE_RANGE) {
            // When we are close to the end, it is cheaper to not create
            // a temporary vector, and swap directly
            if (i + 3 >= len) {
                len -= 1;
                while (i < len) self.swap(i, { i += 1; i });
                self.pop_back()
            } else {
                let other = empty();
                move_range(self, i, 1, &mut other, 0);
                let result = other.pop_back();
                other.destroy_empty();
                result
            }
        } else {
            len -= 1;
            while (i < len) self.swap(i, { i += 1; i });
            self.pop_back()
        }
    }
    spec remove {
        pragma intrinsic = true;
    }

    /// Remove the first occurrence of a given value in the vector `self` and return it in a vector, shifting all
    /// subsequent elements.
    /// This is O(n) and preserves ordering of elements in the vector.
    /// This returns an empty vector if the value isn't present in the vector.
    /// Note that this cannot return an option as option uses vector and there'd be a circular dependency between option
    /// and vector.
    public fun remove_value<Element>(self: &mut vector<Element>, val: &Element): vector<Element> {
        // This doesn't cost a O(2N) run time as index_of scans from left to right and stops when the element is found,
        // while remove would continue from the identified index to the end of the vector.
        let (found, index) = self.index_of(val);
        if (found) {
            vector[self.remove(index)]
        } else {
           vector[]
        }
    }
    spec remove_value {
        pragma intrinsic = true;
    }

    /// Swap the `i`th element of the vector `self` with the last element and then pop the vector.
    /// This is O(1), but does not preserve ordering of elements in the vector.
    /// Aborts if `i` is out of bounds.
    public fun swap_remove<Element>(self: &mut vector<Element>, i: u64): Element {
        assert!(!self.is_empty(), EINDEX_OUT_OF_BOUNDS);
        let last_idx = self.length() - 1;
        self.swap(i, last_idx);
        self.pop_back()
    }
    spec swap_remove {
        pragma intrinsic = true;
    }

    /// Replace the `i`th element of the vector `self` with the given value, and return
    /// to the caller the value that was there before.
    /// Aborts if `i` is out of bounds.
    public fun replace<Element>(self: &mut vector<Element>, i: u64, val: Element): Element {
        let last_idx = self.length();
        assert!(i < last_idx, EINDEX_OUT_OF_BOUNDS);
        if (USE_MOVE_RANGE) {
            mem::replace(self.borrow_mut(i), val)
        } else {
            self.push_back(val);
            self.swap(i, last_idx);
            self.pop_back()
        }
    }

    /// Apply the function to each element in the vector, consuming it.
    public inline fun for_each<Element>(self: vector<Element>, f: |Element|) {
        self.reverse(); // We need to reverse the vector to consume it efficiently
        self.for_each_reverse(|e| f(e));
    }

    /// Apply the function to each element in the vector, consuming it.
    public inline fun for_each_reverse<Element>(self: vector<Element>, f: |Element|) {
        let len = self.length();
        while (len > 0) {
            f(self.pop_back());
            len -= 1;
        };
        self.destroy_empty()
    }

    /// Apply the function to a reference of each element in the vector.
    public inline fun for_each_ref<Element>(self: &vector<Element>, f: |&Element|) {
        let i = 0;
        let len = self.length();
        while (i < len) {
            f(self.borrow(i));
            i += 1
        }
    }

    /// Apply the function to each pair of elements in the two given vectors, consuming them.
    public inline fun zip<Element1, Element2>(self: vector<Element1>, v2: vector<Element2>, f: |Element1, Element2|) {
        // We need to reverse the vectors to consume it efficiently
        self.reverse();
        v2.reverse();
        self.zip_reverse(v2, |e1, e2| f(e1, e2));
    }

    /// Apply the function to each pair of elements in the two given vectors in the reverse order, consuming them.
    /// This errors out if the vectors are not of the same length.
    public inline fun zip_reverse<Element1, Element2>(
        self: vector<Element1>,
        v2: vector<Element2>,
        f: |Element1, Element2|,
    ) {
        let len = self.length();
        // We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(len == v2.length(), 0x20002);
        while (len > 0) {
            f(self.pop_back(), v2.pop_back());
            len -= 1;
        };
        self.destroy_empty();
        v2.destroy_empty();
    }

    /// Apply the function to the references of each pair of elements in the two given vectors.
    /// This errors out if the vectors are not of the same length.
    public inline fun zip_ref<Element1, Element2>(
        self: &vector<Element1>,
        v2: &vector<Element2>,
        f: |&Element1, &Element2|,
    ) {
        let len = self.length();
        // We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(len == v2.length(), 0x20002);
        let i = 0;
        while (i < len) {
            f(self.borrow(i), v2.borrow(i));
            i += 1
        }
    }

    /// Apply the function to a reference of each element in the vector with its index.
    public inline fun enumerate_ref<Element>(self: &vector<Element>, f: |u64, &Element|) {
        let i = 0;
        let len = self.length();
        while (i < len) {
            f(i, self.borrow(i));
            i += 1;
        };
    }

    /// Apply the function to a mutable reference to each element in the vector.
    public inline fun for_each_mut<Element>(self: &mut vector<Element>, f: |&mut Element|) {
        let i = 0;
        let len = self.length();
        while (i < len) {
            f(self.borrow_mut(i));
            i += 1
        }
    }

    /// Apply the function to mutable references to each pair of elements in the two given vectors.
    /// This errors out if the vectors are not of the same length.
    public inline fun zip_mut<Element1, Element2>(
        self: &mut vector<Element1>,
        v2: &mut vector<Element2>,
        f: |&mut Element1, &mut Element2|,
    ) {
        let i = 0;
        let len = self.length();
        // We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(len == v2.length(), 0x20002);
        while (i < len) {
            f(self.borrow_mut(i), v2.borrow_mut(i));
            i += 1
        }
    }

    /// Apply the function to a mutable reference of each element in the vector with its index.
    public inline fun enumerate_mut<Element>(self: &mut vector<Element>, f: |u64, &mut Element|) {
        let i = 0;
        let len = self.length();
        while (i < len) {
            f(i, self.borrow_mut(i));
            i += 1;
        };
    }

    /// Fold the function over the elements. For example, `fold(vector[1,2,3], 0, f)` will execute
    /// `f(f(f(0, 1), 2), 3)`
    public inline fun fold<Accumulator, Element>(
        self: vector<Element>,
        init: Accumulator,
        f: |Accumulator,Element|Accumulator
    ): Accumulator {
        let accu = init;
        self.for_each(|elem| accu = f(accu, elem));
        accu
    }

    /// Fold right like fold above but working right to left. For example, `fold(vector[1,2,3], 0, f)` will execute
    /// `f(1, f(2, f(3, 0)))`
    public inline fun foldr<Accumulator, Element>(
        self: vector<Element>,
        init: Accumulator,
        f: |Element, Accumulator|Accumulator
    ): Accumulator {
        let accu = init;
        self.for_each_reverse(|elem| accu = f(elem, accu));
        accu
    }

    /// Map the function over the references of the elements of the vector, producing a new vector without modifying the
    /// original vector.
    public inline fun map_ref<Element, NewElement>(
        self: &vector<Element>,
        f: |&Element|NewElement
    ): vector<NewElement> {
        let result = vector<NewElement>[];
        self.for_each_ref(|elem| result.push_back(f(elem)));
        result
    }

    /// Map the function over the references of the element pairs of two vectors, producing a new vector from the return
    /// values without modifying the original vectors.
    public inline fun zip_map_ref<Element1, Element2, NewElement>(
        self: &vector<Element1>,
        v2: &vector<Element2>,
        f: |&Element1, &Element2|NewElement
    ): vector<NewElement> {
        // We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(self.length() == v2.length(), 0x20002);

        let result = vector<NewElement>[];
        self.zip_ref(v2, |e1, e2| result.push_back(f(e1, e2)));
        result
    }

    /// Map the function over the elements of the vector, producing a new vector.
    public inline fun map<Element, NewElement>(
        self: vector<Element>,
        f: |Element|NewElement
    ): vector<NewElement> {
        let result = vector<NewElement>[];
        self.for_each(|elem| result.push_back(f(elem)));
        result
    }

    /// Map the function over the element pairs of the two vectors, producing a new vector.
    public inline fun zip_map<Element1, Element2, NewElement>(
        self: vector<Element1>,
        v2: vector<Element2>,
        f: |Element1, Element2|NewElement
    ): vector<NewElement> {
        // We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
        // due to how inline functions work.
        assert!(self.length() == v2.length(), 0x20002);

        let result = vector<NewElement>[];
        self.zip(v2, |e1, e2| result.push_back(f(e1, e2)));
        result
    }

    /// Filter the vector using the boolean function, removing all elements for which `p(e)` is not true.
    public inline fun filter<Element:drop>(
        self: vector<Element>,
        p: |&Element|bool
    ): vector<Element> {
        let result = vector<Element>[];
        self.for_each(|elem| {
            if (p(&elem)) result.push_back(elem);
        });
        result
    }

    /// Partition, sorts all elements for which pred is true to the front.
    /// Preserves the relative order of the elements for which pred is true,
    /// BUT NOT for the elements for which pred is false.
    public inline fun partition<Element>(
        self: &mut vector<Element>,
        pred: |&Element|bool
    ): u64 {
        let i = 0;
        let len = self.length();
        while (i < len) {
            if (!pred(self.borrow(i))) break;
            i += 1;
        };
        let p = i;
        i += 1;
        while (i < len) {
            if (pred(self.borrow(i))) {
                self.swap(p, i);
                p += 1;
            };
            i += 1;
        };
        p
    }

    /// rotate(&mut [1, 2, 3, 4, 5], 2) -> [3, 4, 5, 1, 2] in place, returns the split point
    /// ie. 3 in the example above
    public fun rotate<Element>(
        self: &mut vector<Element>,
        rot: u64
    ): u64 {
        let len = self.length();
        self.rotate_slice(0, rot, len)
    }
    spec rotate {
        pragma intrinsic = true;
    }

    /// Same as above but on a sub-slice of an array [left, right) with left <= rot <= right
    /// returns the
    public fun rotate_slice<Element>(
        self: &mut vector<Element>,
        left: u64,
        rot: u64,
        right: u64
    ): u64 {
        self.reverse_slice(left, rot);
        self.reverse_slice(rot, right);
        self.reverse_slice(left, right);
        left + (right - rot)
    }
    spec rotate_slice {
        pragma intrinsic = true;
    }

    /// Partition the array based on a predicate p, this routine is stable and thus
    /// preserves the relative order of the elements in the two partitions.
    public inline fun stable_partition<Element>(
        self: &mut vector<Element>,
        p: |&Element|bool
    ): u64 {
        let len = self.length();
        let t = empty();
        let f = empty();
        while (len > 0) {
            let e = self.pop_back();
            if (p(&e)) {
                t.push_back(e);
            } else {
                f.push_back(e);
            };
            len -= 1;
        };
        let pos = t.length();
        self.reverse_append(t);
        self.reverse_append(f);
        pos
    }

    /// Return true if any element in the vector satisfies the predicate.
    public inline fun any<Element>(
        self: &vector<Element>,
        p: |&Element|bool
    ): bool {
        let result = false;
        let i = 0;
        while (i < self.length()) {
            result = p(self.borrow(i));
            if (result) {
                break
            };
            i += 1
        };
        result
    }

    /// Return true if all elements in the vector satisfy the predicate.
    public inline fun all<Element>(
        self: &vector<Element>,
        p: |&Element|bool
    ): bool {
        let result = true;
        let i = 0;
        while (i < self.length()) {
            result = p(self.borrow(i));
            if (!result) {
                break
            };
            i += 1
        };
        result
    }

    /// Destroy a vector, just a wrapper around for_each_reverse with a descriptive name
    /// when used in the context of destroying a vector.
    public inline fun destroy<Element>(
        self: vector<Element>,
        d: |Element|
    ) {
        self.for_each_reverse(|e| d(e))
    }

    public fun range(start: u64, end: u64): vector<u64> {
        range_with_step(start, end, 1)
    }

    public fun range_with_step(start: u64, end: u64, step: u64): vector<u64> {
        assert!(step > 0, EINVALID_STEP);

        let vec = vector[];
        while (start < end) {
            vec.push_back(start);
            start += step;
        };
        vec
    }

    public fun slice<Element: copy>(
        self: &vector<Element>,
        start: u64,
        end: u64
    ): vector<Element> {
        assert!(start <= end && end <= self.length(), EINVALID_SLICE_RANGE);

        let vec = vector[];
        while (start < end) {
            vec.push_back(self[start]);
            start += 1;
        };
        vec
    }

    // =================================================================
    // Module Specification

    spec module {} // Switch to module documentation context

    /// # Helper Functions

    spec module {
        /// Check if `self` is equal to the result of adding `e` at the end of `v2`
        fun eq_push_back<Element>(self: vector<Element>, v2: vector<Element>, e: Element): bool {
            len(self) == len(v2) + 1 &&
            self[len(self)-1] == e &&
            self[0..len(self)-1] == v2[0..len(v2)]
        }

        /// Check if `self` is equal to the result of concatenating `v1` and `v2`
        fun eq_append<Element>(self: vector<Element>, v1: vector<Element>, v2: vector<Element>): bool {
            len(self) == len(v1) + len(v2) &&
            self[0..len(v1)] == v1 &&
            self[len(v1)..len(self)] == v2
        }

        /// Check `self` is equal to the result of removing the first element of `v2`
        fun eq_pop_front<Element>(self: vector<Element>, v2: vector<Element>): bool {
            len(self) + 1 == len(v2) &&
            self == v2[1..len(v2)]
        }

        /// Check that `v1` is equal to the result of removing the element at index `i` from `v2`.
        fun eq_remove_elem_at_index<Element>(i: u64, v1: vector<Element>, v2: vector<Element>): bool {
            len(v1) + 1 == len(v2) &&
            v1[0..i] == v2[0..i] &&
            v1[i..len(v1)] == v2[i + 1..len(v2)]
        }

        /// Check if `self` contains `e`.
        fun spec_contains<Element>(self: vector<Element>, e: Element): bool {
            exists x in self: x == e
        }
    }

}
