module 0xc0ffee::m {
    public fun test1_warn() {
        assert!(true);
    }

    public fun test2_warn() {
        assert!(false);
    }

    public fun test3_warn() {
        if (true){
        }else{
            abort(0)
        };
    }

    public fun test4_warn() {
        if (false){
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
        if (true){
        }else{
            abort(0)
        };
    }

    #[lint::skip(assert_const)]
    public fun test4_no_warn() {
        if (false){
        }else{
            abort(0)
        };
    }

    public fun test5_no_warn() {
        let x = true;
        assert!(x);
    }

    public fun test6_no_warn() {
        let x = false;
        assert!(x);
    }
}
