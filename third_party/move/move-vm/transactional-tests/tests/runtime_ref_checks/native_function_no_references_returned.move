//# publish
module 0xc0ffee::m {
    use std::bcs::to_bytes;
    use std::vector;

    fun data_maker(): vector<u8> {
        let data = b"hello world";
        let bytes = to_bytes(&data);
        let len = vector::length(&bytes);
        vector::push_back(&mut bytes, len as u8);
        vector::pop_back(&mut bytes);
        bytes
    }

    public fun test(): u64 {
        let hash = data_maker();
        vector::length(&hash)
    }
}

//# run 0xc0ffee::m::test --verbose
