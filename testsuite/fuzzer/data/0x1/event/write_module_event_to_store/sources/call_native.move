module poc::write_module_event_to_store {
    use aptos_framework::event;

    #[event]
    struct MyEvent has drop, store {
        value: u64
    }

    public entry fun main(_owner: &signer) {
        event::emit<MyEvent>(MyEvent { value: 123 });
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
