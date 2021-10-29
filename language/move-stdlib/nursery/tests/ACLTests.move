#[test_only]
module Std::ACLTests {
    use Std::ACL;
    use Std::Signer;
    use Std::Vector;
    use Std::UnitTest;

    struct Data has key {
        value: u64,
        write_acl: ACL::ACL
    }

    #[test]
    fun test_success() acquires Data {
        let (alice, bob) = create_two_signers();
        let acl = ACL::empty();
        ACL::add(&mut acl, Signer::address_of(&alice));
        move_to(&alice, Data {value: 0, write_acl: acl});

        let alice_data = borrow_global_mut<Data>(Signer::address_of(&alice));
        ACL::add(&mut alice_data.write_acl, Signer::address_of(&bob));
        ACL::remove(&mut alice_data.write_acl, Signer::address_of(&bob));
    }

    #[test]
    #[expected_failure(abort_code = 7)]
    fun test_add_failure() acquires Data {
        let (alice, bob) = create_two_signers();
        let acl = ACL::empty();
        ACL::add(&mut acl, Signer::address_of(&alice));
        move_to(&alice, Data {value: 0, write_acl: acl});

        let alice_data = borrow_global_mut<Data>(Signer::address_of(&alice));
        ACL::add(&mut alice_data.write_acl, Signer::address_of(&bob));
        ACL::add(&mut alice_data.write_acl, Signer::address_of(&bob));
    }

    #[test]
    #[expected_failure(abort_code = 263)]
    fun test_remove_failure() acquires Data {
        let (alice, bob) = create_two_signers();
        let acl = ACL::empty();
        ACL::add(&mut acl, Signer::address_of(&alice));
        move_to(&alice, Data {value: 0, write_acl: acl});

        let alice_data = borrow_global_mut<Data>(Signer::address_of(&alice));
        ACL::remove(&mut alice_data.write_acl, Signer::address_of(&bob));
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
