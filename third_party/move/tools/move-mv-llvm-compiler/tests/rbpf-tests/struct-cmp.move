
// This test file include move-stdlib/sources/vector.move until we are
// able to build move-stdlib.

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

module 0x300::cmp_struct_tests {
    use 0x10::vector as V;

    struct A1 has drop {
        f1: u64
    }

    struct A2 has drop {
        f1: vector<u64>
    }

    struct A3 has drop {
        f1: u8,
        f2: u16,
    }

    struct A4 has drop {
        f1: u8,
        f2: u16,
        f3: u32,
        f4: u64,
        f5: u128,
        f6: u256,
    }

    struct A5 has drop {
        f1: bool,
        f2: vector<u32>
    }

    struct A6 has drop {
        f1: address,
        f2: bool
    }

    public fun doit() {
        let v0a = A1 { f1: 0xffffffffeeeeeeee };
        let v0b = A1 { f1: 0xffffffffeeeeeeee };
        assert!(v0a == v0b, 1);

        assert!(A1 { f1: 22 } != A1 { f1: 23}, 2);

        let v1a = A2 { f1: V::singleton(123) };
        let v1b = A2 { f1: V::singleton(123) };
        assert!(v1a == v1b, 3);

        let v2a = A3 { f1: 0x5a, f2: 0xcafe };
        let v2b = A3 { f1: 0x5a, f2: 0xcafe };
        assert!(v2a == v2b, 4);

        let v3a = A3 { f1: 0x55, f2: 0xcafe };
        let v3b = A3 { f1: 0x5a, f2: 0xcafe };
        assert!(v3a != v3b, 5);

        let v4a = A4 { f1: 0, f2: 2, f3: 3, f4: 4, f5: 5, f6: 6 };
        let v4b = A4 { f1: 1, f2: 2, f3: 3, f4: 4, f5: 5, f6: 6 };
        assert!(v4a != v4b, 6);

        let v5a = A4 { f1: 1, f2: 2, f3: 3, f4: 4, f5: 5, f6: 6 };
        let v5b = A4 { f1: 1, f2: 2, f3: 3, f4: 4, f5: 5, f6: 6 };
        assert!(v5a == v5b, 7);

        let v6a = A5 { f1: true, f2: V::singleton(0xf00dcafe) };
        let v6b = A5 { f1: true, f2: V::singleton(0xf00dcafe) };
        assert!(v6a == v6b, 8);

        let v7a = A5 { f1: false, f2: V::singleton(0xf00dcaff) };
        let v7b = A5 { f1: true, f2: V::singleton(0xf00dcafe) };
        assert!(v7a != v7b, 9);
    }
}

script {
    fun main() {
        use 0x300::cmp_struct_tests as ST;

        ST::doit();
    }
}
