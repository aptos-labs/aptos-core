#[test_only]
module aptos_framework::object_code_deployment_tests {
    use std::signer;
    use aptos_framework::object_code_deployment;
    use aptos_framework::object;

    struct TestProof has drop {}

    #[test(publisher = @0x123)]
    #[expected_failure(abort_code = 0x60003, location = aptos_framework::object_code_deployment)]
    fun test_cannot_register_signer_capability_proof_for_non_code_object(publisher: &signer) {
        object_code_deployment::register_signer_capability_proof<TestProof>(publisher);
    }

    #[test(publisher = @0x123, non_owner = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = aptos_framework::object_code_deployment)]
    fun test_cannot_register_signer_capability_proof_with_non_owner(publisher: &signer, non_owner: &signer) {
        let publisher_address = signer::address_of(publisher);
        let code_object = &object::create_test_object_at(publisher_address, @aptos_framework);
        object_code_deployment::create_fake_code_object(@aptos_framework, object::generate_extend_ref(code_object));

        object_code_deployment::register_signer_capability_proof<TestProof>(non_owner);
    }

    #[test(publisher = @0x123)]
    fun test_register_signer_capability_proof_with_owner(publisher: &signer) {
        let publisher_address = signer::address_of(publisher);
        let code_object = &object::create_test_object_at(publisher_address, @aptos_framework);
        object_code_deployment::create_fake_code_object(@aptos_framework, object::generate_extend_ref(code_object));

        object_code_deployment::register_signer_capability_proof<TestProof>(publisher);
        let code_signer = &object_code_deployment::generate_signer(&TestProof {});
        assert!(signer::address_of(code_signer) == @aptos_framework);
    }
}
