module 0x8675309::M {

    fun borrow_local() {
        &();
        &(1, 2);
        let x = &();
        let y = &(1, 2);
    }

    fun deref_borrow() {
        *&();
        *&(1, 2);
        let x = *&();
        let y = *&(1, 2);
    }

    fun return_tuple() : (u64, u64) {
        return (1, 2)
    }

    fun return_unit() : () {
        return ()
    }

    fun borrow_func() {
        &return_tuple();
        &return_unit();
    }

    inline fun in_unit(): () { () }
    inline fun in_tuple(): (u64, u64) { (1, 2) }

    fun borrow_inline() {
        &in_unit();
        &in_tuple();
    }

    fun borrow_conditional() {
        &(if (true) (1, 2) else (2, 1));
        &(if (false) ());
    }

    fun borrow_special() {
        &(assert!(true, 0));
        &mut spec {};
    }

    fun return_borrow_tuple() : &(u64, u64) {
        return &(1, 2)
    }
}
