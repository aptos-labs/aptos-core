#[test_only]
module Std::ASCIITests {
    use Std::ASCII;
    use Std::Vector;
    use Std::Option;

    #[test]
    fun test_ascii_chars() {
        let i = 0;
        let end = 128;
        let vec = Vector::empty();

        while (i < end) {
            assert!(ASCII::is_valid_char(i), 0);
            Vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let str = ASCII::string(vec);
        assert!(Vector::length(ASCII::as_bytes(&str)) == 128, 0);
        assert!(!ASCII::all_characters_printable(&str), 1);
        assert!(Vector::length(&ASCII::into_bytes(str)) == 128, 2);
    }

    #[test]
    fun test_ascii_push_chars() {
        let i = 0;
        let end = 128;
        let str = ASCII::string(Vector::empty());

        while (i < end) {
            ASCII::push_char(&mut str, ASCII::char(i));
            i = i + 1;
        };

        assert!(Vector::length(ASCII::as_bytes(&str)) == 128, 0);
        assert!(ASCII::length(&str) == 128, 0);
        assert!(!ASCII::all_characters_printable(&str), 1);
    }

    #[test]
    fun test_ascii_push_char_pop_char() {
        let i = 0;
        let end = 128;
        let str = ASCII::string(Vector::empty());

        while (i < end) {
            ASCII::push_char(&mut str, ASCII::char(i));
            i = i + 1;
        };

        while (i > 0) {
            let char = ASCII::pop_char(&mut str);
            assert!(ASCII::byte(char) == i - 1, 0);
            i = i - 1;
        };

        assert!(Vector::length(ASCII::as_bytes(&str)) == 0, 0);
        assert!(ASCII::length(&str) == 0, 0);
        assert!(ASCII::all_characters_printable(&str), 1);
    }

    #[test]
    fun test_printable_chars() {
        let i = 0x20;
        let end = 0x7E;
        let vec = Vector::empty();

        while (i <= end) {
            assert!(ASCII::is_printable_char(i), 0);
            Vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let str = ASCII::string(vec);
        assert!(ASCII::all_characters_printable(&str), 0);
    }

    #[test]
    fun printable_chars_dont_allow_tab() {
        let str = ASCII::string(Vector::singleton(0x09));
        assert!(!ASCII::all_characters_printable(&str), 0);
    }

    #[test]
    fun printable_chars_dont_allow_newline() {
        let str = ASCII::string(Vector::singleton(0x0A));
        assert!(!ASCII::all_characters_printable(&str), 0);
    }

    #[test]
    fun test_invalid_ascii_characters() {
        let i = 128u8;
        let end = 255u8;
        while (i < end) {
            let try_str = ASCII::try_string(Vector::singleton(i));
            assert!(Option::is_none(&try_str), 0);
            i = i + 1;
        };
    }

    #[test]
    fun test_nonvisible_chars() {
        let i = 0;
        let end = 0x09;
        while (i < end) {
            let str = ASCII::string(Vector::singleton(i));
            assert!(!ASCII::all_characters_printable(&str), 0);
            i = i + 1;
        };

        let i = 0x0B;
        let end = 0x0F;
        while (i <= end) {
            let str = ASCII::string(Vector::singleton(i));
            assert!(!ASCII::all_characters_printable(&str), 0);
            i = i + 1;
        };
    }
}
