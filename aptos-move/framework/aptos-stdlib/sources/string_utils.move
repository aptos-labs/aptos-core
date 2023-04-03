module aptos_std::string_utils {
    use std::string::String;

    const EARGS_MISMATCH: u64 = 1;
    const EINVALID_FORMAT: u64 = 2;

    struct Cons<T, N> has copy, drop, store {
        car: T,
        cdr: N,
    }

    struct NIL has copy, drop, store {}

    // Create a pair of values.
    public fun cons<T, N>(car: T, cdr: N): Cons<T, N> {
        Cons { car, cdr }
    }

    // Create a nil value.
    public fun nil(): NIL { NIL {} }

    // Format a move value as a human readable string.
    // eg. format(&1u64) == "1", format(&false) == "false" and format(&cons(1,2)) == "List {car: 1, cdr: 2}"
    public native fun format<T>(s: &T): String;

    // Format a list of move values as a human readable string.
    // eg. format_list(&std::string::utf8(b"a = {} b = {} c = {}"), &cons(1, cons(2, cons(3, nil())))) == "a = 1 b = 2 c = 3"
    public native fun format_list<T>(fmt: &String, val: &T): String;

    #[test]
    fun test_format() {
        assert!(format(&1u64) == std::string::utf8(b"1"), 1);
        assert!(format(&false) == std::string::utf8(b"false"), 2);
        assert!(format(&1u256) == std::string::utf8(b"1"), 3);
        std::debug::print(&format(&cons(1,2)));
        assert!(format(&cons(1,2)) == std::string::utf8(b"Cons {car: 1, cdr: 2}"), 4);
    }

    #[test]
    fun test_format_list() {
        let l = cons(1, cons(2, cons(3, nil())));
        let s = format_list(&std::string::utf8(b"a = {} b = {} c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    #[expected_failure(abort_code = EARGS_MISMATCH)]
    fun test_format_list_to_many_vals() {
        let l = cons(1, cons(2, cons(3, cons(4, nil()))));
        let s = format_list(&std::string::utf8(b"a = {} b = {} c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    #[expected_failure(abort_code = EARGS_MISMATCH)]
    fun test_format_list_not_enough_vals() {
        let l = cons(1, cons(2, nil()));
        let s = format_list(&std::string::utf8(b"a = {} b = {} c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    #[expected_failure(abort_code = EARGS_MISMATCH)]
    fun test_format_list_not_valid_nil() {
        let l = cons(1, cons(2, cons(3, 4)));
        let s = format_list(&std::string::utf8(b"a = {} b = {} c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
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
        let s = format_list(&std::string::utf8(b"a = {} b = {} c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FORMAT)]
    fun test_format_trailing_escape() {
        let l = cons(1, cons(2, cons(3, nil())));
        let s = format_list(&std::string::utf8(b"a = {} b = {} c = {}\\"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FORMAT)]
    fun test_format_unclosed_braces() {
        let l = cons(1, cons(2, cons(3, nil())));
        let s = format_list(&std::string::utf8(b"a = {} b = {} c = {"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FORMAT)]
    fun test_format_unclosed_braces_2() {
        let l = cons(1, cons(2, cons(3, nil())));
        let s = format_list(&std::string::utf8(b"a = {} b = { c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    #[expected_failure(abort_code = EINVALID_FORMAT)]
    fun test_format_unopened_braces() {
        let l = cons(1, cons(2, cons(3, nil())));
        let s = format_list(&std::string::utf8(b"a = } b = {} c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 b = 2 c = 3"), 1);
    }

    #[test]
    fun test_format_escape_escape_works() {
        let l = cons(1, cons(2, cons(3, nil())));
        let s = format_list(&std::string::utf8(b"a = {} \\\\ b = {} c = {}"), &l);
        assert!(s == std::string::utf8(b"a = 1 \\ b = 2 c = 3"), 1);
    }

    #[test]
    fun test_format_escape_braces_works() {
        let l = cons(1, cons(2, cons(3, nil())));
        let s = format_list(&std::string::utf8(b"\\{a = {} b = {} c = {}\\}"), &l);
        assert!(s == std::string::utf8(b"{a = 1 b = 2 c = 3}"), 1);
    }
}
