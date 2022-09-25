module resource_account::core {
    use std::vector;
    use aptos_framework::account::SignerCapability;
    use aptos_framework::resource_account;
    use aptos_std::from_bcs;

    struct ModuleData has key {
        u64_value: u64,
        u8_value: u8,
        address_value: address,
        signer_cap: SignerCapability,
    }

    fun init_module(resource: &signer) {
        let (signer_cap, blob) = resource_account::retrieve_resource_account_cap_and_data(resource, @0xCAFE);
        let u64_blob = vector::empty();
        let idx = 0;
        let end = 8;

        assert!(vector::length(&blob) == 41, vector::length(&blob));

        loop {
            vector::push_back(&mut u64_blob, *vector::borrow(&blob, idx));
            idx = idx + 1;
            if (idx == end) {
                break
            };
        };
        let u64_value = from_bcs::to_u64(u64_blob);

        let u8_value = *vector::borrow(&blob, idx);
        idx = idx + 1;

        let address_blob = vector::empty();
        end = idx + 32;

        loop {
            vector::push_back(&mut address_blob, *vector::borrow(&blob, idx));
            idx = idx + 1;
            if (end == idx) {
                break
            };
        };
        let address_value = from_bcs::to_address(address_blob);

        move_to(
            resource,
            ModuleData {
                u64_value,
                u8_value,
                address_value,
                signer_cap,
            }
        );
    }
}
