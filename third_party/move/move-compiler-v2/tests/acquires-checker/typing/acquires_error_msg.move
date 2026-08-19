module 0x42::test {
    struct Test has key {
        value: u64
    }

    struct Other has key {
        value: u64
    }

    public fun call_modify_without_acquire() acquires Other {
        borrow_global<Other>(@0xcafe);
        modify(); // expect error message here: `Test` acquired via this call
    }

    public fun modify() acquires Test {
        borrow_global_mut<Test>(@0xcafe).value = 2;
    }
}
