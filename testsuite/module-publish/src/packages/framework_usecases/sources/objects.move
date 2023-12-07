
module 0xABCD::objects {
    use std::signer;
    use std::vector;
    use aptos_framework::object;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct AdditionalData has key {
        data: vector<u8>,
    }

    public entry fun create_objects(user: &signer, count: u64, extra_size: u64) {
        let user_address = signer::address_of(user);

        let vec = vector::empty<u8>();
        let i = 0;
        while (i < extra_size) {
            vector::push_back(&mut vec, ((i % 100) as u8));
            i = i + 1;
        };

        while (count > 0) {
            let constructor_ref = object::create_object(user_address);
            if (extra_size > 0) {
                let object_signer = object::generate_signer(&constructor_ref);
                move_to(&object_signer, AdditionalData{data: vec});
            };
            count = count - 1;
        }
    }
}
