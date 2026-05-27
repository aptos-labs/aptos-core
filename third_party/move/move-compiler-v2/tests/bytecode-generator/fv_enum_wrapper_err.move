module 0x66::fv_enum_wrapper {

    enum Wrapper {
         A(||u64 has copy),
         B(||u64 has copy + drop),
    }

    fun call(f: Wrapper): u64
    {
        (f.0)()
    }

    public fun test(): u64
    {
        let a = Wrapper::A(|| 42);
        call(a)
    }

}
