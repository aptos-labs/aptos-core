module 0x42::test {
    struct Test has key {
        value: u64
    }

    public fun call_modify_without_acquire() {
        modify(); // expect error message here
    }

    public fun modify() acquires Test {
        borrow_global_mut<Test>(@0xcafe).value = 2;
    }
}
