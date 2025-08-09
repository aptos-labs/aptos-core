module 0x42::objects {

    struct ReaderRef<phantom T: key> has store {
        addr: address
    }

    public fun get_addr<T: key>(ref: &ReaderRef<T>): address {
        ref.addr
    }

    public inline fun reader<T: key>(ref: &ReaderRef<T>): &T {
        borrow_global<T>(get_addr(ref))
    }
}

module 0x42::token {
    use 0x42::objects as obj;

    struct Token has key { val: u64 }

    public fun get_value(ref: &obj::ReaderRef<Token>): u64 {
        obj::reader(ref).val
    }
}
