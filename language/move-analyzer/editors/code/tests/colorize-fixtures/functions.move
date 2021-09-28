module 0x1::M {
    fun f1() {
        f1();
        M::f1();
    }

    public fun f2<T: copy + drop>(): vector<u8> {
        f2<vector<T>>();
    }
}
