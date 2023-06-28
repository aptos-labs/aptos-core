
// This is move-stdlib/sources/string.move until we build move-stdlib.

/// The `string` module defines the `String` type which represents UTF8 encoded strings.
//module std::string {
module 0x10::string {
    //use std::vector;
    //use std::option::{Self, Option};
    use 0x1::vector;
    use 0x10::option::{Self, Option};

    /// An invalid UTF8 encoding.
    const EINVALID_UTF8: u64 = 1;

    /// Index out of range.
    const EINVALID_INDEX: u64 = 2;

    /// A `String` holds a sequence of bytes which is guaranteed to be in utf8 format.
    struct String has copy, drop, store {
        bytes: vector<u8>,
    }

    /// Creates a new string from a sequence of bytes. Aborts if the bytes do not represent valid utf8.
    public fun utf8(bytes: vector<u8>): String {
        assert!(internal_check_utf8(&bytes), EINVALID_UTF8);
        String{bytes}
    }

    /// Tries to create a new string from a sequence of bytes.
    public fun try_utf8(bytes: vector<u8>): Option<String> {
        if (internal_check_utf8(&bytes)) {
            option::some(String{bytes})
        } else {
            option::none()
        }
    }

    /// Returns a reference to the underlying byte vector.
    public fun bytes(s: &String): &vector<u8> {
        &s.bytes
    }

    /// Checks whether this string is empty.
    public fun is_empty(s: &String): bool {
        vector::is_empty(&s.bytes)
    }

    /// Returns the length of this string, in bytes.
    public fun length(s: &String): u64 {
        vector::length(&s.bytes)
    }

    /// Appends a string.
    public fun append(s: &mut String, r: String) {
        vector::append(&mut s.bytes, r.bytes)
    }

    /// Appends bytes which must be in valid utf8 format.
    public fun append_utf8(s: &mut String, bytes: vector<u8>) {
        append(s, utf8(bytes))
    }

    /// Insert the other string at the byte index in given string. The index must be at a valid utf8 char
    /// boundary.
    public fun insert(s: &mut String, at: u64, o: String) {
        let bytes = &s.bytes;
        assert!(at <= vector::length(bytes) && internal_is_char_boundary(bytes, at), EINVALID_INDEX);
        let l = length(s);
        let front = sub_string(s, 0, at);
        let end = sub_string(s, at, l);
        append(&mut front, o);
        append(&mut front, end);
        *s = front;
    }

    /// Returns a sub-string using the given byte indices, where `i` is the first byte position and `j` is the start
    /// of the first byte not included (or the length of the string). The indices must be at valid utf8 char boundaries,
    /// guaranteeing that the result is valid utf8.
    public fun sub_string(s: &String, i: u64, j: u64): String {
        let bytes = &s.bytes;
        let l = vector::length(bytes);
        assert!(
            j <= l && i <= j && internal_is_char_boundary(bytes, i) && internal_is_char_boundary(bytes, j),
            EINVALID_INDEX
        );
        String{bytes: internal_sub_string(bytes, i, j)}
    }

    /// Computes the index of the first occurrence of a string. Returns `length(s)` if no occurrence found.
    public fun index_of(s: &String, r: &String): u64 {
        internal_index_of(&s.bytes, &r.bytes)
    }


    // Native API
    native fun internal_check_utf8(v: &vector<u8>): bool;
    native fun internal_is_char_boundary(v: &vector<u8>, i: u64): bool;
    native fun internal_sub_string(v: &vector<u8>, i: u64, j: u64): vector<u8>;
    native fun internal_index_of(v: &vector<u8>, r: &vector<u8>): u64;
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


// This file is copied from move-stdlib/sources/option.move
// until we are able to build move-stdlib.

//-----------------------------------------------------------------------------
/// This module defines the Option type and its methods to represent and handle an optional value.
//module std::option {
module 0x10::option {
    //use std::vector;
    use 0x1::vector;

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


module 0x10::string_tests {
    use 0x10::string;

    public fun test_valid_utf8() {
        let sparkle_heart = vector[240, 159, 146, 150];
        let s = string::utf8(sparkle_heart);
        assert!(string::length(&s) == 4, 22);
    }

    public fun test_sub_string() {
        let s = string::utf8(b"abcd");
        let sub = string::sub_string(&s, 2, 4);
        assert!(sub == string::utf8(b"cd"), 22)
    }

    public fun test_sub_string_empty() {
        let s = string::utf8(b"abcd");
        let sub = string::sub_string(&s, 4, 4);
        assert!(string::is_empty(&sub), 22)
    }

    public fun test_index_of() {
        let s = string::utf8(b"abcd");
        let r = string::utf8(b"bc");
        let p = string::index_of(&s, &r);
        assert!(p == 1, 22)
    }

    public fun test_index_of_fail() {
        let s = string::utf8(b"abcd");
        let r = string::utf8(b"bce");
        let p = string::index_of(&s, &r);
        assert!(p == 4, 22)
    }

    public fun test_append() {
        let s = string::utf8(b"abcd");
        string::append(&mut s, string::utf8(b"ef"));
        assert!(s == string::utf8(b"abcdef"), 22)
    }

    public fun test_insert() {
        let s = string::utf8(b"abcd");
        string::insert(&mut s, 1, string::utf8(b"xy"));
        assert!(s == string::utf8(b"axybcd"), 22)
    }
}

script {
    use 0x10::string_tests as ST;

    fun main() {
        ST::test_valid_utf8();
        ST::test_sub_string();
        ST::test_sub_string_empty();
        ST::test_index_of();
        ST::test_index_of_fail();
        ST::test_append();
        ST::test_insert();
    }
}
