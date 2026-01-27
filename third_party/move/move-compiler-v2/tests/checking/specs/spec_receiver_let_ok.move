module 0x42::M {
    struct S {
        value: u64
    }
    fun borrow(self: &S): &S {
        self
    }
    fun get_value(self: &S): u64 {
        self.value
    }

    fun test_borrow(c: S): u64 {
        c.value
    }
    spec test_borrow {
        let s_ref = c.borrow();
        ensures result == s_ref.value;
    }

    fun test_borrow_get_value(data: &S): u64 {
        data.value
    }
    spec test_borrow_get_value {
        let data_value = data.borrow().get_value();
        ensures result == data_value;
    }
}
