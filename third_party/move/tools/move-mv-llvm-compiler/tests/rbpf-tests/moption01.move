
// This test files include move-stdlib/sources/{option.move, vector.move}
// until we are able to build move-stdlib.

//-----------------------------------------------------------------------------
/// This module defines the Option type and its methods to represent and handle an optional value.
//module std::option {
module 0x10::option {
    //use std::vector;
    use 0x10::vector;

    /// Abstraction of a value that may or may not be present. Implemented with a vector of size
    /// zero or one because Move bytecode does not have ADTs.
    struct Option<Element> has copy, drop, store {
        vec: vector<Element>
    }

    /// The `Option` is in an invalid state for the operation attempted.
    /// The `Option` is `Some` while it should be `None`.
    const EOPTION_IS_SET: u64 = 0x40000;
    /// The `Option` is in an invalid state for the operation attempted.
    /// The `Option` is `None` while it should be `Some`.
    const EOPTION_NOT_SET: u64 = 0x40001;

    /// Return an empty `Option`
    public fun none<Element>(): Option<Element> {
        Option { vec: vector::empty() }
    }

    /// Return an `Option` containing `e`
    public fun some<Element>(e: Element): Option<Element> {
        Option { vec: vector::singleton(e) }
    }

    /// Return true if `t` does not hold a value
    public fun is_none<Element>(t: &Option<Element>): bool {
        vector::is_empty(&t.vec)
    }

    /// Return true if `t` holds a value
    public fun is_some<Element>(t: &Option<Element>): bool {
        !vector::is_empty(&t.vec)
    }

    /// Return true if the value in `t` is equal to `e_ref`
    /// Always returns `false` if `t` does not hold a value
    public fun contains<Element>(t: &Option<Element>, e_ref: &Element): bool {
        vector::contains(&t.vec, e_ref)
    }

    /// Return an immutable reference to the value inside `t`
    /// Aborts if `t` does not hold a value
    public fun borrow<Element>(t: &Option<Element>): &Element {
        assert!(is_some(t), EOPTION_NOT_SET);
        vector::borrow(&t.vec, 0)
    }

    /// Return a reference to the value inside `t` if it holds one
    /// Return `default_ref` if `t` does not hold a value
    public fun borrow_with_default<Element>(t: &Option<Element>, default_ref: &Element): &Element {
        let vec_ref = &t.vec;
        if (vector::is_empty(vec_ref)) default_ref
        else vector::borrow(vec_ref, 0)
    }

    /// Return the value inside `t` if it holds one
    /// Return `default` if `t` does not hold a value
    public fun get_with_default<Element: copy + drop>(
        t: &Option<Element>,
        default: Element,
    ): Element {
        let vec_ref = &t.vec;
        if (vector::is_empty(vec_ref)) default
        else *vector::borrow(vec_ref, 0)
    }

    /// Convert the none option `t` to a some option by adding `e`.
    /// Aborts if `t` already holds a value
    public fun fill<Element>(t: &mut Option<Element>, e: Element) {
        let vec_ref = &mut t.vec;
        if (vector::is_empty(vec_ref)) vector::push_back(vec_ref, e)
        else abort EOPTION_IS_SET
    }

    /// Convert a `some` option to a `none` by removing and returning the value stored inside `t`
    /// Aborts if `t` does not hold a value
    public fun extract<Element>(t: &mut Option<Element>): Element {
        assert!(is_some(t), EOPTION_NOT_SET);
        vector::pop_back(&mut t.vec)
    }

    /// Return a mutable reference to the value inside `t`
    /// Aborts if `t` does not hold a value
    public fun borrow_mut<Element>(t: &mut Option<Element>): &mut Element {
        assert!(is_some(t), EOPTION_NOT_SET);
        vector::borrow_mut(&mut t.vec, 0)
    }

    /// Swap the old value inside `t` with `e` and return the old value
    /// Aborts if `t` does not hold a value
    public fun swap<Element>(t: &mut Option<Element>, e: Element): Element {
        assert!(is_some(t), EOPTION_NOT_SET);
        let vec_ref = &mut t.vec;
        let old_value = vector::pop_back(vec_ref);
        vector::push_back(vec_ref, e);
        old_value
    }

    /// Swap the old value inside `t` with `e` and return the old value;
    /// or if there is no old value, fill it with `e`.
    /// Different from swap(), swap_or_fill() allows for `t` not holding a value.
    public fun swap_or_fill<Element>(t: &mut Option<Element>, e: Element): Option<Element> {
        let vec_ref = &mut t.vec;
        let old_value = if (vector::is_empty(vec_ref)) none()
            else some(vector::pop_back(vec_ref));
        vector::push_back(vec_ref, e);
        old_value
    }

    /// Destroys `t.` If `t` holds a value, return it. Returns `default` otherwise
    public fun destroy_with_default<Element: drop>(t: Option<Element>, default: Element): Element {
        let Option { vec } = t;
        if (vector::is_empty(&mut vec)) default
        else vector::pop_back(&mut vec)
    }

    /// Unpack `t` and return its contents
    /// Aborts if `t` does not hold a value
    public fun destroy_some<Element>(t: Option<Element>): Element {
        assert!(is_some(&t), EOPTION_NOT_SET);
        let Option { vec } = t;
        let elem = vector::pop_back(&mut vec);
        vector::destroy_empty(vec);
        elem
    }

    /// Unpack `t`
    /// Aborts if `t` holds a value
    public fun destroy_none<Element>(t: Option<Element>) {
        assert!(is_none(&t), EOPTION_IS_SET);
        let Option { vec } = t;
        vector::destroy_empty(vec)
    }

    /// Convert `t` into a vector of length 1 if it is `Some`,
    /// and an empty vector otherwise
    public fun to_vec<Element>(t: Option<Element>): vector<Element> {
        let Option { vec } = t;
        vec
    }
}

//-----------------------------------------------------------------------------

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

//-----------------------------------------------------------------------------
// These tests are from move-stdlib/tests/option_tests.move.

//module std::option_tests {
module 0x300::option_tests {
    //use std::option;
    use 0x10::option;
    use 0x10::vector;

    public fun option_none_is_none() {
        let none = option::none<u64>();
        assert!(option::is_none(&none), 0);
        assert!(!option::is_some(&none), 1);
    }

    public fun option_some_is_some() {
        let some = option::some(5);
        assert!(!option::is_none(&some), 0);
        assert!(option::is_some(&some), 1);
    }

    public fun option_contains() {
        let none = option::none<u64>();
        let some = option::some(5);
        let some_other = option::some(6);
        assert!(option::contains(&some, &5), 0);
        assert!(option::contains(&some_other, &6), 1);
        assert!(!option::contains(&none, &5), 2);
        assert!(!option::contains(&some_other, &5), 3);
    }

    public fun option_borrow_some() {
        let some = option::some(5);
        let some_other = option::some(6);
        assert!(*option::borrow(&some) == 5, 3);
        assert!(*option::borrow(&some_other) == 6, 4);
    }

    public fun borrow_mut_some() {
        let some = option::some(1);
        let ref = option::borrow_mut(&mut some);
        *ref = 10;
        assert!(*option::borrow(&some) == 10, 0);
    }

    public fun borrow_with_default() {
        let none = option::none<u64>();
        let some = option::some(5);
        assert!(*option::borrow_with_default(&some, &7) == 5, 0);
        assert!(*option::borrow_with_default(&none, &7) == 7, 1);
    }

    public fun get_with_default() {
        let none = option::none<u64>();
        let some = option::some(5);
        assert!(option::get_with_default(&some, 7) == 5, 0);
        assert!(option::get_with_default(&none, 7) == 7, 1);
    }

    public fun extract_some() {
        let opt = option::some(1);
        assert!(option::extract(&mut opt) == 1, 0);
        assert!(option::is_none(&opt), 1);
    }

    public fun swap_some() {
        let some = option::some(5);
        assert!(option::swap(&mut some, 1) == 5, 0);
        assert!(*option::borrow(&some) == 1, 1);
    }

    public fun swap_or_fill_some() {
        let some = option::some(5);
        assert!(option::swap_or_fill(&mut some, 1) == option::some(5), 0);
        assert!(*option::borrow(&some) == 1, 1);
    }

    public fun swap_or_fill_none() {
        let none = option::none();
        assert!(option::swap_or_fill(&mut none, 1) == option::none(), 0);
        assert!(*option::borrow(&none) == 1, 1);
    }

    public fun fill_none() {
        let none = option::none<u64>();
        option::fill(&mut none, 3);
        assert!(option::is_some(&none), 0);
        assert!(*option::borrow(&none) == 3, 1);
    }

    public fun destroy_with_default() {
        assert!(option::destroy_with_default(option::none<u64>(), 4) == 4, 0);
        assert!(option::destroy_with_default(option::some(4), 5) == 4, 1);
    }

    public fun destroy_some() {
        assert!(option::destroy_some(option::some(4)) == 4, 0);
    }

    public fun destroy_none() {
        option::destroy_none(option::none<u64>());
    }

    public fun into_vec_some() {
        let v = option::to_vec(option::some<u64>(0));
        assert!(vector::length(&v) == 1, 0);
        let x = vector::pop_back(&mut v);
        assert!(x == 0, 1);
    }

    public fun into_vec_none() {
        let v: vector<u64> = option::to_vec(option::none());
        assert!(vector::is_empty(&v), 0);
    }
}

script {
    fun main() {
        use 0x300::option_tests as OT;

        OT::option_none_is_none();
        OT::option_some_is_some();
        OT::option_contains();
        OT::option_borrow_some();
        OT::borrow_mut_some();
        OT::borrow_with_default();
        OT::get_with_default();
        OT::extract_some();
        OT::swap_some();
        OT::swap_or_fill_some();
        OT::swap_or_fill_none();
        OT::fill_none();
        OT::destroy_with_default();
        OT::destroy_some();
        OT::destroy_none();
        OT::into_vec_some();
        OT::into_vec_none();
    }
}
