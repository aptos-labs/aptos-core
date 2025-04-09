module poc::exists_at {
    use aptos_framework::object::{Self, Object};
    use std::signer;

    struct MyObject has key {
        value: u64
    }

    public entry fun main(owner: &signer) {
        let constructor_ref = object::create_sticky_object(signer::address_of(owner));
        let object_signer = object::generate_signer(&constructor_ref);
        move_to(&object_signer, MyObject { value: 42 });
        let obj: Object<MyObject> = object::object_from_constructor_ref(&constructor_ref);
        let object_addr = object::object_address(&obj);
        let _exists = object::object_exists<MyObject>(object_addr);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
