#[test_only]
module Std::CapabilityTests {
    use Std::Capability;
    use Std::Signer;
    use Std::UnitTest;
    use Std::Vector;

    struct Feature has drop {}

    #[test]
    fun test_success() {
        let owner = create_signer();
        Capability::create(&owner, &Feature{});
        let _cap = Capability::acquire(&owner, &Feature{});
    }

    #[test]
    #[expected_failure(abort_code = 5)]
    fun test_failure() {
        let (owner, other) = create_two_signers();
        Capability::create(&owner, &Feature{});
        let _cap = Capability::acquire(&other, &Feature{});
    }

    #[test]
    fun test_delegate_success() {
        let (owner, delegate) = create_two_signers();
        Capability::create(&owner, &Feature{});
        let cap = Capability::acquire(&owner, &Feature{});
        Capability::delegate(cap, &Feature{}, &delegate);
        let _delegate_cap = Capability::acquire(&delegate, &Feature{});
    }

    #[test]
    #[expected_failure(abort_code = 5)]
    fun test_delegate_failure_after_revoke() {
        let (owner, delegate) = create_two_signers();
        Capability::create(&owner, &Feature{});
        let cap = Capability::acquire(&owner, &Feature{});
        Capability::delegate(copy cap, &Feature{}, &delegate); // the copy should NOT be needed
        Capability::revoke(cap, &Feature{}, Signer::address_of(&delegate));
        let _delegate_cap = Capability::acquire(&delegate, &Feature{});
    }

    #[test_only]
    fun create_signer(): signer {
        Vector::pop_back(&mut UnitTest::create_signers_for_testing(1))
    }

    #[test_only]
    fun create_two_signers(): (signer, signer) {
        let signers = &mut UnitTest::create_signers_for_testing(2);
        (Vector::pop_back(signers), Vector::pop_back(signers))
    }
}
