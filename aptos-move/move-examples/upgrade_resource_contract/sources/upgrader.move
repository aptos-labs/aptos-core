module upgrade_resource_contract::upgrader {
    use std::signer;
    use std::account::{SignerCapability};
    use std::resource_account;
    use std::account;
    use std::code;

    struct MySignerCapability has key {
        resource_signer_cap: SignerCapability,
    }

    fun init_module(resource_signer: &signer) {
        assert!(signer::address_of(resource_signer) == @upgrade_resource_contract, 0);
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_signer, @owner);
        move_to(resource_signer, MySignerCapability {
            resource_signer_cap: resource_signer_cap,
        });
    }

    // Note the assertion that the caller is @owner. If we leave this line out, anyone can upgrade the contract, exposing the resource account's resources and the contract functionality.
    public entry fun upgrade_contract(
        owner: &signer,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) acquires MySignerCapability {
        assert!(signer::address_of(owner) == @owner, 1);
        let resource_signer_cap = &borrow_global<MySignerCapability>(@upgrade_resource_contract).resource_signer_cap;
        let resource_signer = account::create_signer_with_capability(resource_signer_cap);
        code::publish_package_txn(
            &resource_signer,
            metadata_serialized,
            code,
        );
    }

    #[view]
    public fun upgradeable_function(): u64 {
        9000
    }
}