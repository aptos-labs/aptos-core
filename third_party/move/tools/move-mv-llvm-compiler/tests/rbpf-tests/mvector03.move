
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


// Detailed vector tests move-stdlib/tests/vector_tests.move.
module 0x300::vector_tests {
    use 0x10::vector as V;

    struct R has store { }
    struct Droppable has drop {}
    struct NotDroppable {}

    public fun test_singleton_contains() {
        assert!(*V::borrow(&V::singleton(0), 0) == 0, 0);
        assert!(*V::borrow(&V::singleton(true), 0) == true, 0);
        assert!(*V::borrow(&V::singleton(@0x1), 0) == @0x1, 0);
    }

    public fun test_singleton_len() {
        assert!(V::length(&V::singleton(0)) == 1, 0);
        assert!(V::length(&V::singleton(true)) == 1, 0);
        assert!(V::length(&V::singleton(@0x1)) == 1, 0);
    }

    public fun test_empty_is_empty() {
        assert!(V::is_empty(&V::empty<u64>()), 0);
    }

    public fun append_empties_is_empty() {
        let v1 = V::empty<u64>();
        let v2 = V::empty<u64>();
        V::append(&mut v1, v2);
        assert!(V::is_empty(&v1), 0);
    }

    public fun append_respects_order_empty_lhs() {
        let v1 = V::empty();
        let v2 = V::empty();
        V::push_back(&mut v2, 0);
        V::push_back(&mut v2, 1);
        V::push_back(&mut v2, 2);
        V::push_back(&mut v2, 3);
        V::append(&mut v1, v2);
        assert!(!V::is_empty(&v1), 0);
        assert!(V::length(&v1) == 4, 1);
        assert!(*V::borrow(&v1, 0) == 0, 2);
        assert!(*V::borrow(&v1, 1) == 1, 3);
        assert!(*V::borrow(&v1, 2) == 2, 4);
        assert!(*V::borrow(&v1, 3) == 3, 5);
    }

    public fun append_respects_order_empty_rhs() {
        let v1 = V::empty();
        let v2 = V::empty();
        V::push_back(&mut v1, 0);
        V::push_back(&mut v1, 1);
        V::push_back(&mut v1, 2);
        V::push_back(&mut v1, 3);
        V::append(&mut v1, v2);
        assert!(!V::is_empty(&v1), 0);
        assert!(V::length(&v1) == 4, 1);
        assert!(*V::borrow(&v1, 0) == 0, 2);
        assert!(*V::borrow(&v1, 1) == 1, 3);
        assert!(*V::borrow(&v1, 2) == 2, 4);
        assert!(*V::borrow(&v1, 3) == 3, 5);
    }

    public fun append_respects_order_nonempty_rhs_lhs() {
        let v1 = V::empty();
        let v2 = V::empty();
        V::push_back(&mut v1, 0);
        V::push_back(&mut v1, 1);
        V::push_back(&mut v1, 2);
        V::push_back(&mut v1, 3);
        V::push_back(&mut v2, 4);
        V::push_back(&mut v2, 5);
        V::push_back(&mut v2, 6);
        V::push_back(&mut v2, 7);
        V::append(&mut v1, v2);
        assert!(!V::is_empty(&v1), 0);
        assert!(V::length(&v1) == 8, 1);
        let i = 0;
        while (i < 8) {
            assert!(*V::borrow(&v1, i) == i, i);
            i = i + 1;
        }
    }

    public fun vector_contains() {
        let vec = V::empty();
        assert!(!V::contains(&vec, &0), 1);

        V::push_back(&mut vec, 0);
        assert!(V::contains(&vec, &0), 2);
        assert!(!V::contains(&vec, &1), 3);

        V::push_back(&mut vec, 1);
        assert!(V::contains(&vec, &0), 4);
        assert!(V::contains(&vec, &1), 5);
        assert!(!V::contains(&vec, &2), 6);

        V::push_back(&mut vec, 2);
        assert!(V::contains(&vec, &0), 7);
        assert!(V::contains(&vec, &1), 8);
        assert!(V::contains(&vec, &2), 9);
        assert!(!V::contains(&vec, &3), 10);
    }

    public fun destroy_empty() {
        V::destroy_empty(V::empty<u64>());
        // TODO: V::destroy_empty(V::empty<R>());
    }

    public fun destroy_empty_with_pops() {
        let v = V::empty();
        V::push_back(&mut v, 42);
        V::pop_back(&mut v);
        V::destroy_empty(v);
    }

    public fun get_set_work() {
        let vec = V::empty();
        V::push_back(&mut vec, 0);
        V::push_back(&mut vec, 1);
        assert!(*V::borrow(&vec, 1) == 1, 0);
        assert!(*V::borrow(&vec, 0) == 0, 1);

        *V::borrow_mut(&mut vec, 0) = 17;
        assert!(*V::borrow(&vec, 1) == 1, 0);
        assert!(*V::borrow(&vec, 0) == 17, 0);
    }

    public fun swap_different_indices() {
        let vec = V::empty();
        V::push_back(&mut vec, 0);
        V::push_back(&mut vec, 1);
        V::push_back(&mut vec, 2);
        V::push_back(&mut vec, 3);
        V::swap(&mut vec, 0, 3);
        V::swap(&mut vec, 1, 2);
        assert!(*V::borrow(&vec, 0) == 3, 0);
        assert!(*V::borrow(&vec, 1) == 2, 0);
        assert!(*V::borrow(&vec, 2) == 1, 0);
        assert!(*V::borrow(&vec, 3) == 0, 0);
    }

    public fun swap_same_index() {
        let vec = V::empty();
        V::push_back(&mut vec, 0);
        V::push_back(&mut vec, 1);
        V::push_back(&mut vec, 2);
        V::push_back(&mut vec, 3);
        V::swap(&mut vec, 1, 1);
        assert!(*V::borrow(&vec, 0) == 0, 0);
        assert!(*V::borrow(&vec, 1) == 1, 0);
        assert!(*V::borrow(&vec, 2) == 2, 0);
        assert!(*V::borrow(&vec, 3) == 3, 0);
    }

    public fun remove_singleton_vector() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        assert!(V::remove(&mut v, 0) == 0, 0);
        assert!(V::length(&v) == 0, 0);
    }

    public fun remove_nonsingleton_vector() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        V::push_back(&mut v, 1);
        V::push_back(&mut v, 2);
        V::push_back(&mut v, 3);

        assert!(V::remove(&mut v, 1) == 1, 0);
        assert!(V::length(&v) == 3, 0);
        assert!(*V::borrow(&v, 0) == 0, 0);
        assert!(*V::borrow(&v, 1) == 2, 0);
        assert!(*V::borrow(&v, 2) == 3, 0);
    }

    public fun remove_nonsingleton_vector_last_elem() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        V::push_back(&mut v, 1);
        V::push_back(&mut v, 2);
        V::push_back(&mut v, 3);

        assert!(V::remove(&mut v, 3) == 3, 0);
        assert!(V::length(&v) == 3, 0);
        assert!(*V::borrow(&v, 0) == 0, 0);
        assert!(*V::borrow(&v, 1) == 1, 0);
        assert!(*V::borrow(&v, 2) == 2, 0);
    }

    public fun reverse_vector_empty() {
        let v = V::empty<u64>();
        let is_empty = V::is_empty(&v);
        V::reverse(&mut v);
        assert!(is_empty == V::is_empty(&v), 0);
    }

    public fun reverse_singleton_vector() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        assert!(*V::borrow(&v, 0) == 0, 1);
        V::reverse(&mut v);
        assert!(*V::borrow(&v, 0) == 0, 2);
    }

    public fun reverse_vector_nonempty_even_length() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        V::push_back(&mut v, 1);
        V::push_back(&mut v, 2);
        V::push_back(&mut v, 3);

        assert!(*V::borrow(&v, 0) == 0, 1);
        assert!(*V::borrow(&v, 1) == 1, 2);
        assert!(*V::borrow(&v, 2) == 2, 3);
        assert!(*V::borrow(&v, 3) == 3, 4);

        V::reverse(&mut v);

        assert!(*V::borrow(&v, 3) == 0, 5);
        assert!(*V::borrow(&v, 2) == 1, 6);
        assert!(*V::borrow(&v, 1) == 2, 7);
        assert!(*V::borrow(&v, 0) == 3, 8);
    }

    public fun reverse_vector_nonempty_odd_length_non_singleton() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        V::push_back(&mut v, 1);
        V::push_back(&mut v, 2);

        assert!(*V::borrow(&v, 0) == 0, 1);
        assert!(*V::borrow(&v, 1) == 1, 2);
        assert!(*V::borrow(&v, 2) == 2, 3);

        V::reverse(&mut v);

        assert!(*V::borrow(&v, 2) == 0, 4);
        assert!(*V::borrow(&v, 1) == 1, 5);
        assert!(*V::borrow(&v, 0) == 2, 6);
    }

    public fun swap_remove_singleton() {
        let v = V::empty<u64>();
        V::push_back(&mut v, 0);
        assert!(V::swap_remove(&mut v, 0) == 0, 0);
        assert!(V::is_empty(&v), 1);
    }

    public fun swap_remove_inside_vector() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        V::push_back(&mut v, 1);
        V::push_back(&mut v, 2);
        V::push_back(&mut v, 3);

        assert!(*V::borrow(&v, 0) == 0, 1);
        assert!(*V::borrow(&v, 1) == 1, 2);
        assert!(*V::borrow(&v, 2) == 2, 3);
        assert!(*V::borrow(&v, 3) == 3, 4);

        assert!(V::swap_remove(&mut v, 1) == 1, 5);
        assert!(V::length(&v) == 3, 6);

        assert!(*V::borrow(&v, 0) == 0, 7);
        assert!(*V::borrow(&v, 1) == 3, 8);
        assert!(*V::borrow(&v, 2) == 2, 9);

    }

    public fun swap_remove_end_of_vector() {
        let v = V::empty();
        V::push_back(&mut v, 0);
        V::push_back(&mut v, 1);
        V::push_back(&mut v, 2);
        V::push_back(&mut v, 3);

        assert!(*V::borrow(&v, 0) == 0, 1);
        assert!(*V::borrow(&v, 1) == 1, 2);
        assert!(*V::borrow(&v, 2) == 2, 3);
        assert!(*V::borrow(&v, 3) == 3, 4);

        assert!(V::swap_remove(&mut v, 3) == 3, 5);
        assert!(V::length(&v) == 3, 6);

        assert!(*V::borrow(&v, 0) == 0, 7);
        assert!(*V::borrow(&v, 1) == 1, 8);
        assert!(*V::borrow(&v, 2) == 2, 9);
    }

    public fun push_back_and_borrow() {
        let v = V::empty();
        V::push_back(&mut v, 7);
        assert!(!V::is_empty(&v), 0);
        assert!(V::length(&v) == 1, 1);
        assert!(*V::borrow(&v, 0) == 7, 2);

        V::push_back(&mut v, 8);
        assert!(V::length(&v) == 2, 3);
        assert!(*V::borrow(&v, 0) == 7, 4);
        assert!(*V::borrow(&v, 1) == 8, 5);
    }

    public fun index_of_empty_not_has() {
        let v = V::empty();
        let (has, index) = V::index_of(&v, &true);
        assert!(!has, 0);
        assert!(index == 0, 1);
    }

    public fun index_of_nonempty_not_has() {
        let v = V::empty();
        V::push_back(&mut v, false);
        let (has, index) = V::index_of(&v, &true);
        assert!(!has, 0);
        assert!(index == 0, 1);
    }

    public fun index_of_nonempty_has() {
        let v = V::empty();
        V::push_back(&mut v, false);
        V::push_back(&mut v, true);
        let (has, index) = V::index_of(&v, &true);
        assert!(has, 0);
        assert!(index == 1, 1);
    }

    // index_of will return the index first occurence that is equal
    public fun index_of_nonempty_has_multiple_occurences() {
        let v = V::empty();
        V::push_back(&mut v, false);
        V::push_back(&mut v, true);
        V::push_back(&mut v, true);
        let (has, index) = V::index_of(&v, &true);
        assert!(has, 0);
        assert!(index == 1, 1);
    }

    public fun length() {
        let empty = V::empty();
        assert!(V::length(&empty) == 0, 0);
        let i = 0;
        let max_len = 42;
        while (i < max_len) {
            V::push_back(&mut empty, i);
            assert!(V::length(&empty) == i + 1, i);
            i = i + 1;
        }
    }

    public fun pop_push_back() {
        let v = V::empty();
        let i = 0;
        let max_len = 42;

        while (i < max_len) {
            V::push_back(&mut v, i);
            i = i + 1;
        };

        while (i > 0) {
            assert!(V::pop_back(&mut v) == i - 1, i);
            i = i - 1;
        };
    }

    fun test_natives_with_type<T>(x1: T, x2: T): (T, T) {
        let v = V::empty();
        assert!(V::length(&v) == 0, 0);
        V::push_back(&mut v, x1);
        assert!(V::length(&v) == 1, 1);
        V::push_back(&mut v, x2);
        assert!(V::length(&v) == 2, 2);
        V::swap(&mut v, 0, 1);
        x1 = V::pop_back(&mut v);
        assert!(V::length(&v) == 1, 3);
        x2 = V::pop_back(&mut v);
        assert!(V::length(&v) == 0, 4);
        V::destroy_empty(v);
        (x1, x2)
    }

    public fun test_natives_with_different_instantiations() {
        test_natives_with_type<u8>(1u8, 2u8);
        test_natives_with_type<u16>(45356u16, 25345u16);
        test_natives_with_type<u32>(45356u32, 28768867u32);
        test_natives_with_type<u64>(1u64, 2u64);
        test_natives_with_type<u128>(1u128, 2u128);
        // TODO U256  test_natives_with_type<u256>(45356u256, 253458768867u256);
        test_natives_with_type<bool>(true, false);
        test_natives_with_type<address>(@0x1, @0x2);

        test_natives_with_type<vector<u8>>(V::empty(), V::empty());

        // TODO_STRUCT  test_natives_with_type<Droppable>(Droppable{}, Droppable{});
        //(NotDroppable {}, NotDroppable {}) = test_natives_with_type<NotDroppable>(
        //    NotDroppable {},
        //    NotDroppable {}
        //);
    }

    public fun test_insert() {
        let v = vector[7];
        V::insert(&mut v, 6, 0);
        assert!(v == vector[6, 7], 0);

        let v = vector[7, 9];
        V::insert(&mut v, 8, 1);
        assert!(v == vector[7, 8, 9], 0);

        let v = vector[6, 7];
        V::insert(&mut v, 5, 0);
        assert!(v == vector[5, 6, 7], 0);

        let v = vector[5, 6, 8];
        V::insert(&mut v, 7, 2);
        assert!(v == vector[5, 6, 7, 8], 0);
    }

    public fun insert_at_end() {
        let v = vector[];
        V::insert(&mut v, 6, 0);
        assert!(v == vector[6], 0);

        V::insert(&mut v, 7, 1);
        assert!(v == vector[6, 7], 0);
    }
}

script {
    fun main() {
        use 0x300::vector_tests as VT;

        VT::test_singleton_contains();
        VT::test_singleton_len();
        VT::test_empty_is_empty();
        VT::append_empties_is_empty();
        VT::append_respects_order_empty_lhs();
        VT::append_respects_order_empty_rhs();
        VT::append_respects_order_nonempty_rhs_lhs();
        VT::vector_contains();
        VT::destroy_empty();
        VT::destroy_empty_with_pops();
        VT::get_set_work();
        VT::swap_different_indices();
        VT::swap_same_index();
        VT::remove_singleton_vector();
        VT::remove_nonsingleton_vector();
        VT::remove_nonsingleton_vector_last_elem();
        VT::reverse_vector_empty();
        VT::reverse_singleton_vector();
        VT::reverse_vector_nonempty_even_length();
        VT::reverse_vector_nonempty_odd_length_non_singleton();
        VT::swap_remove_inside_vector();
        VT::swap_remove_end_of_vector();
        VT::push_back_and_borrow();
        VT::index_of_empty_not_has();
        VT::index_of_nonempty_not_has();
        VT::index_of_nonempty_has();
        VT::index_of_nonempty_has_multiple_occurences();
        VT::length();
        VT::pop_push_back();
        VT::test_natives_with_different_instantiations();
        VT::test_insert();
        VT::insert_at_end();
    }
}
