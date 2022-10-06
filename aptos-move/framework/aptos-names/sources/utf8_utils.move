module aptos_names::utf8_utils {

    use std::string::{Self, String};
    use std::vector;

    /// Happens if the bytes do not start with {0, 110, 1110, 11110}
    const EINVALID_UTF8_START: u64 = 0;

    const LATIN_LOWER_ALPHA_CHARSET_SIZE: u64 = 26;
    const LATIN_DIGIT_CHARSET_SIZE: u64 = 10;

    const CHARACTER_SET_LATIN_LOWER_ALPHA_IDX: u64 = 0;
    const CHARACTER_SET_LATIN_DIGIT_IDX: u64 = 1;

    /// Only allow latin lowercase letters, digits, and hyphens
    /// Hyphens are not allowed at the beginning or end of a string
    /// Returns whether it is allowed, and the number of characters in the utf8 string
    fun bytes_are_allowed(bytes: &vector<u8>): (bool, u64) {
        let u64_characters = utf8_to_vec_u64_fast(bytes);
        let i = 0;
        let len = vector::length(&u64_characters);
        let allowed = true;

        while (i < len) {
            let c = *vector::borrow(&u64_characters, i);

            // latin hyphen
            if (c == 45) {
                if (i == 0 || i == len - 1) {
                    // hyphen at beginning or end is not allowed
                    return (false, len)
                };
                // We ignore hyphens from the character set count, it's easy to determine later
            }
            // latin numbers 0-9
            else if (c >= 48 && c <= 57) {
                // these are valid
            }
            // latin lowercase letters a-z
            else if (c >= 97 && c <= 122) {
                // these are valid
            } else {
                // uknown character set: this is not valid
                return (false, len)
            };
            i = i + 1;
        };

        (allowed, len)
    }

    /// A convenience function for `bytes_are_allowed`
    /// Returns whether it is allowed, number of characters in the utf8 string, and number of distinct charsets used
    public fun string_is_allowed(string: &String): (bool, u64) {
        bytes_are_allowed(string::bytes(string))
    }

    /// This parses a UTF-8 string into a vector of u64s, where each u64 is the full character.
    /// This includes all control bits, and so is faster due to skipping some bit operations.
    /// This will validate:
    ///     - invalid bytes
    /// This will skip validation of the following, but this should not cause character collissions:
    ///     - an unexpected continuation byte
    ///     - a non-continuation byte before the end of the character
    ///     - the string ending before the end of the character (which can happen in simple string truncation)
    /// an overlong encoding
    /// a sequence that decodes to an invalid code point
    /// For more information on the estructure of UTF8: https://en.wikipedia.org/wiki/UTF-8#Encoding
    public fun utf8_to_vec_u64_fast(bytes: &vector<u8>): vector<u64> {
        let len = vector::length(bytes);
        let i = 0;
        let result = vector::empty<u64>();
        while (i < len) {
            let char1 = *vector::borrow(bytes, i);
            let prefix: u8 = char1 >> 4;
            if (prefix < 8) {
                // 0xxx xxxx
                vector::push_back(&mut result, (char1 as u64));
            } else if (prefix == 12 || prefix == 13) {
                // 110x xxxx  10xx xxxx
                let char1 = (char1 as u64);
                let char2 = (*vector::borrow(bytes, i + 1) as u64);
                vector::push_back(&mut result, (char1 << 8) | char2);
                i = i + 1;
            } else if (prefix == 14) {
                // 1110 xxxx  10xx xxxx  10xx xxxx
                let char1 = (char1 as u64);
                let char2 = (*vector::borrow(bytes, i + 1) as u64);
                let char3 = (*vector::borrow(bytes, i + 2) as u64);
                vector::push_back(&mut result, (char1 << 16) | (char2 << 8) | char3);
                i = i + 2;
            } else if (prefix == 15) {
                // 1111 0xxx  10xx xxxx  10xx xxxx  10xx xxxx
                let char1 = (char1 as u64);
                let char2 = (*vector::borrow(bytes, i + 1) as u64);
                let char3 = (*vector::borrow(bytes, i + 2) as u64);
                let char4 = (*vector::borrow(bytes, i + 3) as u64);
                vector::push_back(&mut result, (char1 << 24) | (char2 << 16) | (char3 << 8) | char4);
                i = i + 3;
            } else {
                assert!(char1 <= 14u8, EINVALID_UTF8_START);
            };
            i = i + 1;
        };
        result
    }

    /// This turns a u128 into its UTF-8 string equivalent.
    public fun u128_to_string(value: u128): String {
        if (value == 0) {
            return string::utf8(b"0")
        };
        let buffer = vector::empty<u8>();
        while (value != 0) {
            vector::push_back(&mut buffer, ((48 + value % 10) as u8));
            value = value / 10;
        };
        vector::reverse(&mut buffer);
        string::utf8(buffer)
    }

    #[test_only]
    struct Example has copy, drop {
        text: vector<u8>,
        length: u64,
    }

    #[test]
    fun test_latin_digits() {
        let allowed_tests: vector<Example> = vector[
            Example { text: b"01234-56789", length: 11, },
            Example { text: b"abcdefgh-ijklmnopqrstuvwxyz", length: 27, },
            Example { text: b"a", length: 1, },
            Example { text: b"", length: 0, },
        ];
        // Reverse it so the errors are in order
        vector::reverse(&mut allowed_tests);
        let i = 0;
        let len = vector::length(&allowed_tests);
        while (i < len) {
            let example = vector::pop_back(&mut allowed_tests);
            let (was_allowed, length) = bytes_are_allowed(&example.text);
            assert!(was_allowed, i);
            assert!(length == example.length, i);
            i = i + 1;
        };

        // The char_counts here should only count up to the first invalid character
        let not_allowed: vector<Example> = vector[
            Example { text: b"a_a", length: 3, },
            Example { text: b"-aaa", length: 4, },
            Example { text: b"aaa_", length: 4, },
            Example { text: b"-", length: 1, },
            Example { text: b"_", length: 1, },
            Example { text: b"a!b", length: 3, },
            Example { text: b"A", length: 1, },
        ];
        // Reverse it so the errors are in order
        vector::reverse(&mut not_allowed);
        let i = 0;
        let len = vector::length(&not_allowed);
        while (i < len) {
            let example = vector::pop_back(&mut not_allowed);
            let (was_allowed, length) = bytes_are_allowed(&example.text);
            assert!(!was_allowed, i);
            assert!(length == example.length, i);
            i = i + 1;
        };
    }

    #[test]
    fun test_utf8_to_vec_u64_fast() {
        // https://unicode-table.com/en/0053/
        let english_capital_s: vector<u8> = vector[0x53];
        let res1 = utf8_to_vec_u64_fast(&english_capital_s);
        assert!(res1 == vector[83], vector::pop_back(&mut res1));

        // https://unicode-table.com/en/05D0/
        let hebrew_alef: vector<u8> = vector[0xD7, 0x90];
        let res2 = utf8_to_vec_u64_fast(&hebrew_alef);
        assert!(res2 == vector[55184], vector::pop_back(&mut res2));

        // https://unicode-table.com/en/0E12/
        let thai_tho_phuthao: vector<u8> = vector[0xE0, 0xB8, 0x92];
        let res2 = utf8_to_vec_u64_fast(&thai_tho_phuthao);
        assert!(res2 == vector[14727314], vector::pop_back(&mut res2));

        // https://unicode-table.com/en/1F496/
        let sparkle_heart: vector<u8> = vector[0xF0, 0x9F, 0x92, 0x96];
        let res4 = utf8_to_vec_u64_fast(&sparkle_heart);
        assert!(res4 == vector[4036989590], vector::pop_back(&mut res4));
    }
}
