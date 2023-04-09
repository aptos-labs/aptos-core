module aptos_std::string_utils {
    use std::string::String;

    /// The number of values in the list does not match the number of "{}" in the format string.
    const EARGS_MISMATCH: u64 = 1;
    /// The format string is not valid.
    const EINVALID_FORMAT: u64 = 2;

    struct Cons<T, N> has copy, drop, store {
        car: T,
        cdr: N,
    }

    struct NIL has copy, drop, store {}

    /// Create a pair of values.
    public fun cons<T, N>(car: T, cdr: N): Cons<T, N> { Cons { car, cdr } }

    /// Create a nil value.
    public fun nil(): NIL { NIL {} }

    /// Specialized versions of format_list for 1, 2, 3 and 4 values as a convenience.
    public fun format1<T0: drop>(fmt: &vector<u8>, a: T0): String {
        native_format_list(fmt, &list1(a))
    }
    public fun format2<T0: drop, T1: drop>(fmt: &vector<u8>, a: T0, b: T1): String {
        native_format_list(fmt, &list2(a, b))
    }
    public fun format3<T0: drop, T1: drop, T2: drop>(fmt: &vector<u8>, a: T0, b: T1, c: T2): String {
        native_format_list(fmt, &list3(a, b, c))
    }
    public fun format4<T0: drop, T1: drop, T2: drop, T3: drop>(fmt: &vector<u8>, a: T0, b: T1, c: T2, d: T3): String {
        native_format_list(fmt, &list4(a, b, c, d))
    }

    /// Format a move value as a human readable string.
    public fun to_string<T>(s: &T): String {
        native_format(s, false, false, true, false)
    }

    /// Format addresses as 64 zero-padded hexadecimals.
    public fun to_string_with_canonical_addresses<T>(s: &T): String {
        native_format(s, false, true, true, false)
    }

    /// Format emitting integers with types ie. 6u8 or 128u32.
    public fun to_string_with_integer_types<T>(s: &T): String {
        native_format(s, false, true, true, false)
    }

    /// Format vectors and structs with newlines
    public fun debug_string<T>(s: &T): String {
        native_format(s, true, false, false, false)
    }

    /// Format a move value as a human readable string.
    /// eg. `format(&1u64) == "1"`, `format(&false) == "false"` and `format(&cons(1,2)) == "Cons { car: 1, cdr: 2 }"`
    native fun native_format<T>(s: &T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;

    /// Format a list of move values as a human readable string with rust-like format string.
    /// eg. `format_list(&b"a = {} b = {} c = {}", &cons(1, cons(2, cons(3, nil())))) == "a = 1 b = 2 c = 3"`
    /// fmt must be utf8 encoded and must contain the same number of "{}" as the number of values in the list.
    native fun native_format_list<T>(fmt: &vector<u8>, val: &T): String;

    inline fun list1<T0>(a: T0): Cons<T0, NIL> { cons(a, nil()) }
    inline fun list2<T0, T1>(a: T0, b: T1): Cons<T0, Cons<T1, NIL>> { cons(a, list1(b)) }
    inline fun list3<T0, T1, T2>(a: T0, b: T1, c: T2): Cons<T0, Cons<T1, Cons<T2, NIL>>> { cons(a, list2(b, c)) }
    inline fun list4<T0, T1, T2, T3>(a: T0, b: T1, c: T2, d: T3): Cons<T0, Cons<T1, Cons<T2, Cons<T3, NIL>>>> { cons(a, list3(b, c, d)) }

    #[test]
    fun test_format() {
        assert!(to_string(&1u64) == std::string::utf8(b"1"), 1);
        assert!(to_string(&false) == std::string::utf8(b"false"), 2);
        assert!(to_string(&1u256) == std::string::utf8(b"1"), 3);
        assert!(to_string(&vector[1, 2, 3]) == std::string::utf8(b"[ 1, 2, 3 ]"), 4);
        assert!(to_string(&cons(std::string::utf8(b"My string"),2)) == std::string::utf8(b"Cons { car: \"My string\", cdr: 2 }"), 5);
        assert!(to_string(&std::option::none<u64>()) == std::string::utf8(b"None"), 6);
        assert!(to_string(&std::option::some(1)) == std::string::utf8(b"Some(1)"), 7);
    }

    #[test]
    fun test_format_list() {
        let s = format3(&b"a = {} b = {} c = {}", 1, 2, std::string::utf8(b"My string"));
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = \"My string\""), 1);
    }

    #[test]
    #[expected_failure(abort_code = EARGS_MISMATCH)]
    fun test_format_list_to_many_vals() {
        format4(&b"a = {} b = {} c = {}", 1, 2, 3, 4);
    }

    #[test]
    #[expected_failure(abort_code = EARGS_MISMATCH)]
    fun test_format_list_not_enough_vals() {
        format2(&b"a = {} b = {} c = {}", 1, 2);
    }

    #[test]
    #[expected_failure(abort_code = EARGS_MISMATCH)]
    fun test_format_list_not_valid_nil() {
        let l = cons(1, cons(2, cons(3, 4)));
        native_format_list(&b"a = {} b = {} c = {}", &l);
    }

    #[testonly]
    struct FakeCons<T, N> has copy, drop, store {
        car: T,
        cdr: N,
    }

    #[test]
    #[expected_failure(abort_code = EARGS_MISMATCH)]
    fun test_format_list_not_valid_list() {
        let l = cons(1, FakeCons { car: 2, cdr: cons(3, nil())});
        native_format_list(&b"a = {} b = {} c = {}", &l);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FORMAT)]
    fun test_format_unclosed_braces() {
        format3(&b"a = {} b = {} c = {", 1, 2 ,3);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FORMAT)]
    fun test_format_unclosed_braces_2() {
        format3(&b"a = {} b = { c = {}", 1, 2, 3);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FORMAT)]
    fun test_format_unopened_braces() {
        format3(&b"a = } b = {} c = {}", 1, 2, 3);
    }

    #[test]
    fun test_format_escape_braces_works() {
        let s = format3(&b"{{a = {} b = {} c = {}}}", 1, 2, 3);
        assert!(s == std::string::utf8(b"{a = 1 b = 2 c = 3}"), 1);
    }
}
