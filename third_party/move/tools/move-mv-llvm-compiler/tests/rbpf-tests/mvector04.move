
// This is move-stdlib/sources/vector.move minus all the spec stuff to make
// more concise.
//
// Also commented out the #[bytecode_instruction] lines, in which the Move
// compiler converts native functions to bytecode instructions. That won't
// work with our current scheme of translating natives, which expects to
// see calls and produce declarations for them.
//
//module std::vector {
module 0x10::vector {
    /// The index into the vector is out of bounds
    const EINDEX_OUT_OF_BOUNDS: u64 = 0x20000;

    //#[bytecode_instruction]
    /// Create an empty vector.
    native public fun empty<Element>(): vector<Element>;

    //#[bytecode_instruction]
    /// Return the length of the vector.
    native public fun length<Element>(v: &vector<Element>): u64;

    //#[bytecode_instruction]
    /// Acquire an immutable reference to the `i`th element of the vector `v`.
    /// Aborts if `i` is out of bounds.
    native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;

    //#[bytecode_instruction]
    /// Add element `e` to the end of the vector `v`.
    native public fun push_back<Element>(v: &mut vector<Element>, e: Element);

    //#[bytecode_instruction]
    /// Return a mutable reference to the `i`th element in the vector `v`.
    /// Aborts if `i` is out of bounds.
    native public fun borrow_mut<Element>(v: &mut vector<Element>, i: u64): &mut Element;

    //#[bytecode_instruction]
    /// Pop an element from the end of vector `v`.
    /// Aborts if `v` is empty.
    native public fun pop_back<Element>(v: &mut vector<Element>): Element;

    //#[bytecode_instruction]
    /// Destroy the vector `v`.
    /// Aborts if `v` is not empty.
    native public fun destroy_empty<Element>(v: vector<Element>);

    //#[bytecode_instruction]
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


module 0x300::vector_tests {
    use 0x10::vector as V;

    public fun more_vec_literals_u16() {
        let v = vector[7u16];
        V::insert(&mut v, 6, 0);
        assert!(v == vector[6, 7], 0);

        let v = vector[7u16, 9u16];
        V::insert(&mut v, 8, 1);
        assert!(v == vector[7, 8, 9], 0);

        let v = vector[6u16, 7u16];
        V::insert(&mut v, 5, 0);
        assert!(v == vector[5, 6, 7], 0);

        let v = vector[5u16, 6u16, 8u16];
        V::insert(&mut v, 7, 2);
        assert!(v == vector[5, 6, 7, 8], 0);
    }

    public fun more_vec_literals_u32() {
        let v = vector[7u32];
        V::insert(&mut v, 6, 0);
        assert!(v == vector[6, 7], 0);

        let v = vector[7u32, 9u32];
        V::insert(&mut v, 8, 1);
        assert!(v == vector[7, 8, 9], 0);

        let v = vector[6u32, 7u32];
        V::insert(&mut v, 5, 0);
        assert!(v == vector[5, 6, 7], 0);

        let v = vector[5u32, 6u32, 8u32];
        V::insert(&mut v, 7, 2);
        assert!(v == vector[5, 6, 7, 8], 0);
    }

    public fun more_vec_literals_u128() {
        let v = vector[7u128];
        V::insert(&mut v, 6, 0);
        assert!(v == vector[6, 7], 0);

        let v = vector[7u128, 9u128];
        V::insert(&mut v, 8, 1);
        assert!(v == vector[7, 8, 9], 0);

        let v = vector[6u128, 7u128];
        V::insert(&mut v, 5, 0);
        assert!(v == vector[5, 6, 7], 0);

        let v = vector[5u128, 6u128, 8u128];
        V::insert(&mut v, 7, 2);
        assert!(v == vector[5, 6, 7, 8], 0);
    }

    public fun more_vec_literals_u256() {
        let v = vector[7u256];
        V::insert(&mut v, 6, 0);
        assert!(v == vector[6, 7], 0);

        let v = vector[7u256, 9u256];
        V::insert(&mut v, 8, 1);
        assert!(v == vector[7, 8, 9], 0);

        let v = vector[6u256, 7u256];
        V::insert(&mut v, 5, 0);
        assert!(v == vector[5, 6, 7], 0);

        let v = vector[5u256, 6u256, 8u256];
        V::insert(&mut v, 7, 2);
        assert!(v == vector[5, 6, 7, 8], 0);
    }

    public fun more_vec_literals_address() {
        let v = vector[@0x7];
        V::insert(&mut v, @0x6, 0);
        assert!(v == vector[@0x6, @0x7], 0);

        let v = vector[@0x7, @0x9];
        V::insert(&mut v, @0x8, 1);
        assert!(v == vector[@0x7, @0x8, @0x9], 0);

        let v = vector[@0x6, @0x7];
        V::insert(&mut v, @0x5, 0);
        assert!(v == vector[@0x5, @0x6, @0x7], 0);

        let v = vector[@0x5, @0x6, @0x8];
        V::insert(&mut v, @0x7, 2);
        assert!(v == vector[@0x5, @0x6, @0x7, @0x8], 0);
    }

}

script {
    fun main() {
        use 0x300::vector_tests as VT;

        VT::more_vec_literals_u16();
        VT::more_vec_literals_u32();
        VT::more_vec_literals_u128();
        VT::more_vec_literals_u256();
        VT::more_vec_literals_address();
    }
}
