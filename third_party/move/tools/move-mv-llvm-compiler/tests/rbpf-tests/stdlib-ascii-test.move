
// This is move-stdlib/sources/ascii.move until we build move-stdlib.

/// The `ASCII` module defines basic string and char newtypes in Move that verify
/// that characters are valid ASCII, and that strings consist of only valid ASCII characters.
//module std::ascii {
module 0x10::ascii {
    use 0x1::vector;
    use 0x10::option::{Self, Option};

    /// An invalid ASCII character was encountered when creating an ASCII string.
    const EINVALID_ASCII_CHARACTER: u64 = 0x10000;

   /// The `String` struct holds a vector of bytes that all represent
   /// valid ASCII characters. Note that these ASCII characters may not all
   /// be printable. To determine if a `String` contains only "printable"
   /// characters you should use the `all_characters_printable` predicate
   /// defined in this module.
   struct String has copy, drop, store {
       bytes: vector<u8>,
   }
   spec String {
       invariant forall i in 0..len(bytes): is_valid_char(bytes[i]);
   }

   /// An ASCII character.
   struct Char has copy, drop, store {
       byte: u8,
   }
   spec Char {
       invariant is_valid_char(byte);
   }

    /// Convert a `byte` into a `Char` that is checked to make sure it is valid ASCII.
    public fun char(byte: u8): Char {
        assert!(is_valid_char(byte), EINVALID_ASCII_CHARACTER);
        Char { byte }
    }
    spec char {
        aborts_if !is_valid_char(byte) with EINVALID_ASCII_CHARACTER;
    }

    /// Convert a vector of bytes `bytes` into an `String`. Aborts if
    /// `bytes` contains non-ASCII characters.
    public fun string(bytes: vector<u8>): String {
       let x = try_string(bytes);
       assert!(
            option::is_some(&x),
            EINVALID_ASCII_CHARACTER
       );
       option::destroy_some(x)
    }
    spec string {
        aborts_if exists i in 0..len(bytes): !is_valid_char(bytes[i]) with EINVALID_ASCII_CHARACTER;
    }

    /// Convert a vector of bytes `bytes` into an `String`. Returns
    /// `Some(<ascii_string>)` if the `bytes` contains all valid ASCII
    /// characters. Otherwise returns `None`.
    public fun try_string(bytes: vector<u8>): Option<String> {
       let len = vector::length(&bytes);
       let i = 0;
       while ({
           spec {
               invariant i <= len;
               invariant forall j in 0..i: is_valid_char(bytes[j]);
           };
           i < len
       }) {
           let possible_byte = *vector::borrow(&bytes, i);
           if (!is_valid_char(possible_byte)) return option::none();
           i = i + 1;
       };
       spec {
           assert i == len;
           assert forall j in 0..len: is_valid_char(bytes[j]);
       };
       option::some(String { bytes })
    }

    /// Returns `true` if all characters in `string` are printable characters
    /// Returns `false` otherwise. Not all `String`s are printable strings.
    public fun all_characters_printable(string: &String): bool {
       let len = vector::length(&string.bytes);
       let i = 0;
       while ({
           spec {
               invariant i <= len;
               invariant forall j in 0..i: is_printable_char(string.bytes[j]);
           };
           i < len
       }) {
           let byte = *vector::borrow(&string.bytes, i);
           if (!is_printable_char(byte)) return false;
           i = i + 1;
       };
       spec {
           assert i == len;
           assert forall j in 0..len: is_printable_char(string.bytes[j]);
       };
       true
    }
    spec all_characters_printable {
        ensures result ==> (forall j in 0..len(string.bytes): is_printable_char(string.bytes[j]));
    }

    public fun push_char(string: &mut String, char: Char) {
        vector::push_back(&mut string.bytes, char.byte);
    }
    spec push_char {
        ensures len(string.bytes) == len(old(string.bytes)) + 1;
    }

    public fun pop_char(string: &mut String): Char {
        Char { byte: vector::pop_back(&mut string.bytes) }
    }
    spec pop_char {
        ensures len(string.bytes) == len(old(string.bytes)) - 1;
    }

    public fun length(string: &String): u64 {
        vector::length(as_bytes(string))
    }

    /// Get the inner bytes of the `string` as a reference
    public fun as_bytes(string: &String): &vector<u8> {
       &string.bytes
    }

    /// Unpack the `string` to get its backing bytes
    public fun into_bytes(string: String): vector<u8> {
       let String { bytes } = string;
       bytes
    }

    /// Unpack the `char` into its underlying byte.
    public fun byte(char: Char): u8 {
       let Char { byte } = char;
       byte
    }

    /// Returns `true` if `b` is a valid ASCII character. Returns `false` otherwise.
    public fun is_valid_char(b: u8): bool {
       b <= 0x7F
    }

    /// Returns `true` if `byte` is an printable ASCII character. Returns `false` otherwise.
    public fun is_printable_char(byte: u8): bool {
       byte >= 0x20 && // Disallow metacharacters
       byte <= 0x7E // Don't allow DEL metacharacter
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


module 0x10::ascii_tests {
    //use std::ascii;
    //use std::vector;
    //use std::option;
    use 0x10::ascii;
    use 0x1::vector;
    use 0x10::option;

    public fun test_ascii_chars() {
        let i = 0;
        let end = 128;
        let vec = vector::empty();

        while (i < end) {
            assert!(ascii::is_valid_char(i), 0);
            vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let str = ascii::string(vec);
        assert!(vector::length(ascii::as_bytes(&str)) == 128, 0);
        assert!(!ascii::all_characters_printable(&str), 1);
        assert!(vector::length(&ascii::into_bytes(str)) == 128, 2);
    }

    public fun test_ascii_push_chars() {
        let i = 0;
        let end = 128;
        let str = ascii::string(vector::empty());

        while (i < end) {
            ascii::push_char(&mut str, ascii::char(i));
            i = i + 1;
        };

        assert!(vector::length(ascii::as_bytes(&str)) == 128, 0);
        assert!(ascii::length(&str) == 128, 0);
        assert!(!ascii::all_characters_printable(&str), 1);
    }

    public fun test_ascii_push_char_pop_char() {
        let i = 0;
        let end = 128;
        let str = ascii::string(vector::empty());

        while (i < end) {
            ascii::push_char(&mut str, ascii::char(i));
            i = i + 1;
        };

        while (i > 0) {
            let char = ascii::pop_char(&mut str);
            assert!(ascii::byte(char) == i - 1, 0);
            i = i - 1;
        };

        assert!(vector::length(ascii::as_bytes(&str)) == 0, 0);
        assert!(ascii::length(&str) == 0, 0);
        assert!(ascii::all_characters_printable(&str), 1);
    }

    public fun test_printable_chars() {
        let i = 0x20;
        let end = 0x7E;
        let vec = vector::empty();

        while (i <= end) {
            assert!(ascii::is_printable_char(i), 0);
            vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let str = ascii::string(vec);
        assert!(ascii::all_characters_printable(&str), 0);
    }

    public fun printable_chars_dont_allow_tab() {
        let str = ascii::string(vector::singleton(0x09));
        assert!(!ascii::all_characters_printable(&str), 0);
    }

    public fun printable_chars_dont_allow_newline() {
        let str = ascii::string(vector::singleton(0x0A));
        assert!(!ascii::all_characters_printable(&str), 0);
    }

    public fun test_invalid_ascii_characters() {
        let i = 128u8;
        let end = 255u8;
        while (i < end) {
            let try_str = ascii::try_string(vector::singleton(i));
            assert!(option::is_none(&try_str), 0);
            i = i + 1;
        };
    }

    public fun test_nonvisible_chars() {
        let i = 0;
        let end = 0x09;
        while (i < end) {
            let str = ascii::string(vector::singleton(i));
            assert!(!ascii::all_characters_printable(&str), 0);
            i = i + 1;
        };

        let i = 0x0B;
        let end = 0x0F;
        while (i <= end) {
            let str = ascii::string(vector::singleton(i));
            assert!(!ascii::all_characters_printable(&str), 0);
            i = i + 1;
        };
    }
}

script {
    use 0x10::ascii_tests as AT;

    fun main() {
        AT::test_ascii_chars();
        AT::test_ascii_push_chars();
        AT::test_ascii_push_char_pop_char();
        AT::test_printable_chars();
        AT::printable_chars_dont_allow_tab();
        AT::printable_chars_dont_allow_newline();
        AT::test_invalid_ascii_characters();
        AT::test_nonvisible_chars();
    }
}
