module 0x42::m1 {

    public enum Result<T: copy + drop, E: copy +drop> has copy, drop {
        Ok(T),
        Err(E)
    }

}

//# publish
module 0x42::m2 {

    use 0x42::m1::Result;

    public fun test_pack_unpack_abort() {
        let result = Result::Ok<u64, u64>(42);
        assert!(result == Result::Ok(42), 1);
        let Result::Err(err) = result;
        assert!(err == 42, 2);
    }

}
