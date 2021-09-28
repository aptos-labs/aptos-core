module 0x1::M {
    struct S has copy, drop { f: u64 }
    fun f<T: key + store>(p: X<U: copy + drop>) {
        copy p;
        move p;
    }
}
