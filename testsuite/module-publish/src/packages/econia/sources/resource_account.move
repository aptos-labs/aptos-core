/// Manages an Econia-owned resource account.
module econia::resource_account {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::timestamp;
    use std::bcs;

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Friends >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    friend econia::incentives;
    friend econia::market;

    // Friends <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Stores a signing capability for the Econia resource account.
    struct SignerCapabilityStore has key {
        /// Signer capability for Econia resource account.
        signer_capability: SignerCapability
    }

    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // View functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[view]
    /// Return resource account address.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public(friend) fun get_address():
    address
    acquires SignerCapabilityStore {
        // Immutably borrow signer capability.
        let signer_capability_ref =
            &borrow_global<SignerCapabilityStore>(@econia).signer_capability;
        // Return its address.
        account::get_signer_capability_address(signer_capability_ref)
    }

    // View functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public friend functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Return resource account signer.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    public(friend) fun get_signer():
    signer
    acquires SignerCapabilityStore {
        // Immutably borrow signer capability.
        let signer_capability_ref =
            &borrow_global<SignerCapabilityStore>(@econia).signer_capability;
        // Return associated signer.
        account::create_signer_with_capability(signer_capability_ref)
    }

    // Public friend functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Private functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Initialize the Econia resource account upon module publication.
    ///
    /// # Seed considerations
    ///
    /// Uses block timestamp as a seed for future-proofing the potential
    /// creation of additional resource accounts via the Econia account:
    /// the use of a seed that is not hard-coded mitigates the threat
    /// of resource account creation blockage via mismanaged seeds,
    /// assuming in this case that multiple resource accounts are not
    /// created during the same block.
    ///
    /// # Testing
    ///
    /// * `test_mixed()`
    fun init_module(
        econia: &signer
    ) {
        // Get resource account time seed.
        let time_seed = bcs::to_bytes(&timestamp::now_microseconds());
        // Create resource account, storing signer capability.
        let (_, signer_capability) =
            account::create_resource_account(econia, time_seed);
        // Store signing capability under Econia account.
        move_to(econia, SignerCapabilityStore{signer_capability});
    }

    // Private functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    struct TestStruct has key {}

    // Test-only structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// Initialize resource account for testing.
    public fun init_test() {
        // Get signer for Aptos framework account.
        let aptos_framework = account::create_signer_with_capability(
            &account::create_test_signer_cap(@aptos_framework));
        // Set time for seed.
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        // Get signer for Econia account.
        let econia = account::create_signer_with_capability(
            &account::create_test_signer_cap(@econia));
        init_module(&econia); // Init resource account.
    }

    // Test-only functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test]
    /// Verify initialization, signer use, address lookup.
    fun test_mixed()
    acquires SignerCapabilityStore {
        init_test(); // Init the resource account.
        // Move to resource account a test struct.
        move_to<TestStruct>(&get_signer(), TestStruct{});
        // Verify existence via address lookup.
        assert!(exists<TestStruct>(get_address()), 0);
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}