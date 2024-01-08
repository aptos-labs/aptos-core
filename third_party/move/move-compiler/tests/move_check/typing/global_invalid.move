module 0x42::m {

    fun invalid<T:key + drop>(addr: address) {
        assert!(exists<T>(addr), 0);
        let _ = borrow_global<T>(addr);
        move_from<T>(addr);
    }

}
