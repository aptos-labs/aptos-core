#[test_only]
module std::reflect_test {
    use std::string;
    use std::reflect;

    public fun increment(x: u64): u64 { x + 1 }

    fun private_increment(x: u64): u64 { x + 1 }

    public fun generic<T: drop>(_x: T): u64 { 32 }

    public fun generic_cant_infer<T>(_x: u64): u64 { 32 }

    struct S {} // no copy
    public fun generic_copy<T:copy>(r: &T): T { *r }

    #[test]
    fun basic_success() {
        let fn : |u64|u64 = reflect::resolve(@std, &string::utf8(b"reflect_test"), &string::utf8(b"increment")).unwrap();
        assert!(fn(1) == 2, 33)
    }

    #[test]
    fun invalid_ident() {
        let e = reflect::resolve< |u64|u64 >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"su%")).unwrap_err();
        assert!(e.error_code() == 0)
    }

    #[test]
    fun function_not_found() {
        let e = reflect::resolve< |u64|u64 >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"xyz")).unwrap_err();
        assert!(e.error_code() == 1)
    }

    #[test]
    fun module_not_found() {
        let e = reflect::resolve< |u64|u64 >(
            @std, &string::utf8(b"reflect_unknown"), &string::utf8(b"increment")).unwrap_err();
        assert!(e.error_code() == 1)
    }


    #[test]
    fun not_accessible() {
        let e = reflect::resolve< |u64|u64 >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"private_increment")).unwrap_err();
        assert!(e.error_code() == 2)
    }

    #[test]
    fun incompat_type() {
        let e = reflect::resolve< |u64| >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"increment")).unwrap_err();
        assert!(e.error_code() == 3)
    }

    #[test]
    fun generic_success() {
        let fn = reflect::resolve< |u64|u64 >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"generic")).unwrap();
        assert!(fn(2) == 32)
    }

    #[test]
    fun generic_copy_success() {
        let fn = reflect::resolve< |&u64|u64 >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"generic_copy")).unwrap();
        assert!(fn(&2) == 2)
    }

    #[test]
    fun generic_copy_missing_ability() {
        let e = reflect::resolve< |&S|u64 >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"generic_copy")).unwrap_err();
        assert!(e.error_code() == 3)
    }

    #[test]
    fun cannot_infer() {
        let e = reflect::resolve< |u64|u64 >(
            @std, &string::utf8(b"reflect_test"), &string::utf8(b"generic_cant_infer")).unwrap_err();
        assert!(e.error_code() == 4)
    }
}
