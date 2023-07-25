module upgradeable_resource_account_package::basic_contract {
    use upgradeable_resource_account_package::package_manager;
    use std::error;
    use std::signer;

    struct SomeResource has key {
        value: u64,
    }

    /// You are not authorized to perform this action.
    const ENOT_AUTHORIZED: u64 = 0;

    #[view]
    public fun upgradeable_function(): u64 {
        9000
    }

    // An example of doing something with the resource account that requires its signer
    public entry fun move_to_resource_account(deployer: &signer) {
        // Only the deployer can call this function.
        assert!(signer::address_of(deployer) == @deployer, error::permission_denied(ENOT_AUTHORIZED));

        // Do something with the resource account's signer
        // For example, a simple `move_to` call
        let resource_signer = package_manager::get_signer();
        move_to(
            &resource_signer,
            SomeResource {
                value: 42,
            }
        );
    }
}
