spec velor_std::string_utils {
    spec to_string<T>(s: &T): String {
        aborts_if false;
        ensures result == spec_native_format(s, false, false, true, false);
    }

    spec to_string_with_canonical_addresses<T>(s: &T): String {
        aborts_if false;
        ensures result == spec_native_format(s, false, true, true, false);
    }

    spec to_string_with_integer_types<T>(s: &T): String {
        aborts_if false;
        ensures result == spec_native_format(s, false, true, true, false);
    }

    spec debug_string<T>(s: &T): String {
        aborts_if false;
        ensures result == spec_native_format(s, true, false, false, false);
    }

    spec format1<T0: drop>(fmt: &vector<u8>, a: T0): String {
        aborts_if args_mismatch_or_invalid_format(fmt, list1(a));
        ensures result == spec_native_format_list(fmt, list1(a));
    }

    spec format2<T0: drop, T1: drop>(fmt: &vector<u8>, a: T0, b: T1): String {
        aborts_if args_mismatch_or_invalid_format(fmt, list2(a, b));
        ensures result == spec_native_format_list(fmt, list2(a, b));
    }

    spec format3<T0: drop, T1: drop, T2: drop>(fmt: &vector<u8>, a: T0, b: T1, c: T2): String {
        aborts_if args_mismatch_or_invalid_format(fmt, list3(a, b, c));
        ensures result == spec_native_format_list(fmt, list3(a, b, c));
    }

    spec format4<T0: drop, T1: drop, T2: drop, T3: drop>(fmt: &vector<u8>, a: T0, b: T1, c: T2, d: T3): String {
        aborts_if args_mismatch_or_invalid_format(fmt, list4(a, b, c, d));
        ensures result == spec_native_format_list(fmt, list4(a, b, c, d));
    }

    spec native_format<T>(s: &T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String {
        pragma opaque;
        aborts_if false;
        ensures result == spec_native_format(s, type_tag, canonicalize, single_line, include_int_types);
    }

    spec native_format_list<T>(fmt: &vector<u8>, val: &T): String {
        pragma opaque;
        aborts_if args_mismatch_or_invalid_format(fmt, val);
        ensures result == spec_native_format_list(fmt, val);
    }

    spec fun spec_native_format<T>(s: T, type_tag: bool, canonicalize: bool, single_line: bool, include_int_types: bool): String;
    spec fun spec_native_format_list<T>(fmt: vector<u8>, val: T): String;
    spec fun args_mismatch_or_invalid_format<T>(fmt: vector<u8>, val: T): bool;
}
