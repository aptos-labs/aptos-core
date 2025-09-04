
module 0xABCD::objects {
    use std::error;
    use std::signer;
    use std::vector;
    use velor_framework::object;

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    struct AdditionalData has key {
        data: vector<u8>,
    }

    public entry fun create_objects(user: &signer, count: u64, object_payload_size: u64) {
        let user_address = signer::address_of(user);

        let vec = vector::empty<u8>();
        let i = 0;
        while (i < object_payload_size) {
            vector::push_back(&mut vec, ((i % 100) as u8));
            i = i + 1;
        };

        while (count > 0) {
            let constructor_ref = object::create_object(user_address);
            if (object_payload_size > 0) {
                let object_signer = object::generate_signer(&constructor_ref);
                move_to(&object_signer, AdditionalData{data: vec});
            };
            count = count - 1;
        }
    }

    // Resource being modified doesn't exist
    const ECOUNTER_RESOURCE_NOT_PRESENT: u64 = 1;
    const ENOT_AUTHORIZED: u64 = 2;

    struct Counter has key {
        count: u64,
    }

    // Create the global `Counter`.
    // Stored under the module publisher address.
    fun init_module(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @publisher_address,
            ENOT_AUTHORIZED,
        );
        move_to<Counter>(
            publisher,
            Counter { count: 0 },
        );
    }

    public entry fun create_objects_conflict(user: &signer, count: u64, object_payload_size: u64, conflict_address: address) acquires Counter {
        assert!(exists<Counter>(conflict_address), error::invalid_argument(ECOUNTER_RESOURCE_NOT_PRESENT));
        let counter = borrow_global_mut<Counter>(conflict_address);
        counter.count = counter.count + 1;

        create_objects(user, count, object_payload_size);
    }
}
