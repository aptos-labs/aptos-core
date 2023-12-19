/// test_point: has ability{'copy', 'drop', 'store'} with comment

/// The `ASCII` module defines basic string and char newtypes in Move that verify
/// that characters are valid ASCII, and that strings consist of only valid ASCII characters.
module std::ascii {
    use std::vector;
    use std::option::{Self, Option};

    /// The `String` struct holds a vector of bytes that all represent
    /// valid ASCII characters. Note that these ASCII characters may not all
    /// be printable. To determine if a `String` contains only "printable"
    /// characters you should use the `all_characters_printable` predicate
    /// defined in this module.
    struct String has /*comment*/ copy, drop /*comment*/ , store /*comment*/ {
        // comment
        bytes: vector<u8>, // comment

    }

    spec String {
        invariant forall i in 0..len(bytes): is_valid_char(bytes[i]);
    }
}