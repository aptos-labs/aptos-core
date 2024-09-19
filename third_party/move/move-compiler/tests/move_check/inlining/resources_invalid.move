module 0x42::objects {

    struct ReaderRef<phantom T: key> has store {
        addr: address
    }

    public inline fun reader<T: key>(ref: &ReaderRef<T>): &T {
        borrow_global<T>(ref.addr)
    }
}

module 0x42::token {
    use 0x42::objects as obj;

    struct Token has key { val: u64 }

    public fun get_value(ref: &obj::ReaderRef<Token>): u64 acquires Token {
        obj::reader(ref).val
    }
}
