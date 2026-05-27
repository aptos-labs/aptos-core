module 0xc0ffee::m {
    const CONSTANT_TRUE: bool = true;
    const CONSTANT_FALSE: bool = false;

    public fun test1_warn() {
        assert!(true);
    }

    public fun test2_warn() {
        assert!(false);
    }

    public fun test3_warn() {
        assert!(CONSTANT_TRUE);
    }

    public fun test4_warn() {
        assert!(CONSTANT_FALSE);
    }

    public fun test5_warn() {
        assert!(true, 42);
    }

    public fun test6_warn() {
        assert!(false, 42);
    }

    public fun test7_warn() {
        assert!(CONSTANT_TRUE, 42);
    }

    public fun test8_warn() {
        assert!(CONSTANT_FALSE, 42);
    }

    public fun test9_warn() {
        if (true){
        }else{
            abort(0)
        };
    }

    public fun test10_warn() {
        if (false){
        }else{
            abort(0)
        };
    }

    public fun test11_warn() {
        if (CONSTANT_TRUE){
        }else{
            abort(0)
        };
    }

    public fun test12_warn() {
        if (CONSTANT_FALSE){
        }else{
            abort(0)
        };
    }

    #[lint::skip(assert_const)]
    public fun test1_no_warn() {
        assert!(true);
    }

    #[lint::skip(assert_const)]
    public fun test2_no_warn() {
        assert!(false);
    }

    #[lint::skip(assert_const)]
    public fun test3_no_warn() {
        assert!(CONSTANT_TRUE);
    }

    #[lint::skip(assert_const)]
    public fun test4_no_warn() {
        assert!(CONSTANT_FALSE);
    }

    #[lint::skip(assert_const)]
    public fun test5_no_warn() {
        assert!(true, 42);
    }

    #[lint::skip(assert_const)]
    public fun test6_no_warn() {
        assert!(false, 42);
    }

    #[lint::skip(assert_const)]
    public fun test7_no_warn() {
        assert!(CONSTANT_TRUE, 42);
    }

    #[lint::skip(assert_const)]
    public fun test8_no_warn() {
        assert!(CONSTANT_FALSE, 42);
    }

    #[lint::skip(assert_const)]
    public fun test9_no_warn() {
        if (true){
        }else{
            abort(0)
        };
    }

    #[lint::skip(assert_const)]
    public fun test10_no_warn() {
        if (false){
        }else{
            abort(0)
        };
    }

    #[lint::skip(assert_const)]
    public fun test11_no_warn() {
        if (CONSTANT_TRUE){
        }else{
            abort(0)
        };
    }

    #[lint::skip(assert_const)]
    public fun test12_no_warn() {
        if (CONSTANT_FALSE){
        }else{
            abort(0)
        };
    }
}
