/// The `string` module defines the `String` type which represents UTF8 encoded strings.
module std::string {
    use std::option::{Self, Option};

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
    public fun bytes(self: &String): &vector<u8> {
        &self.bytes
    }

    /// Checks whether this string is empty.
    public fun is_empty(self: &String): bool {
        self.bytes.is_empty()
    }

    /// Returns the length of this string, in bytes.
    public fun length(self: &String): u64 {
        self.bytes.length()
    }

    /// Appends a string.
    public fun append(self: &mut String, r: String) {
        self.bytes.append(r.bytes)
    }

    /// Appends bytes which must be in valid utf8 format.
    public fun append_utf8(self: &mut String, bytes: vector<u8>) {
        self.append(utf8(bytes))
    }

    /// Insert the other string at the byte index in given string. The index must be at a valid utf8 char
    /// boundary.
    public fun insert(self: &mut String, at: u64, o: String) {
        let bytes = &self.bytes;
        assert!(at <= bytes.length() && internal_is_char_boundary(bytes, at), EINVALID_INDEX);
        let l = self.length();
        let front = self.sub_string(0, at);
        let end = self.sub_string(at, l);
        front.append(o);
        front.append(end);
        *self = front;
    }

    /// Returns a sub-string using the given byte indices, where `i` is the first byte position and `j` is the start
    /// of the first byte not included (or the length of the string). The indices must be at valid utf8 char boundaries,
    /// guaranteeing that the result is valid utf8.
    public fun sub_string(self: &String, i: u64, j: u64): String {
        let bytes = &self.bytes;
        let l = bytes.length();
        assert!(
            j <= l && i <= j && internal_is_char_boundary(bytes, i) && internal_is_char_boundary(bytes, j),
            EINVALID_INDEX
        );
        String { bytes: internal_sub_string(bytes, i, j) }
    }

    /// Computes the index of the first occurrence of a string. Returns `length(s)` if no occurrence found.
    public fun index_of(self: &String, r: &String): u64 {
        internal_index_of(&self.bytes, &r.bytes)
    }

    // Native API
    public native fun internal_check_utf8(v: &vector<u8>): bool;
    native fun internal_is_char_boundary(v: &vector<u8>, i: u64): bool;
    native fun internal_sub_string(v: &vector<u8>, i: u64, j: u64): vector<u8>;
    native fun internal_index_of(v: &vector<u8>, r: &vector<u8>): u64;
}
