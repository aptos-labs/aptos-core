module 0x1::event {
    public fun emit<T: drop + store>(_msg: T) {
        abort 0
    }
}

module 0x42::m {
    use 0x1::event;

    #[event]
    struct MyEvent has drop, store {
        x: u64,
    }

    public fun test() {
        event::emit(MyEvent { x: 42 });
    }
}
