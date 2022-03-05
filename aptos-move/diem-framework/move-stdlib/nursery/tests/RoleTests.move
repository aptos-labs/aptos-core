#[test_only]
module Std::RoleTests {
    use Std::Role;
    use Std::Vector;
    use Std::UnitTest;

    struct Developer has drop {}
    struct User has drop {}
    struct Admin has drop {}

    #[test]
    fun test_success() {
        let (alice, bob) = create_two_signers();
        Role::assign_role<Developer>(&alice, &Developer{});
        Role::assign_role<User>(&alice, &User{});
        Role::assign_role<Admin>(&bob, &Admin{});

        Role::revoke_role<Developer>(&alice, &Developer{});
        Role::revoke_role<User>(&alice, &User{});
        Role::revoke_role<Admin>(&bob, &Admin{});
    }

    #[test]
    #[expected_failure(abort_code = 6)]
    fun test_assign_failure() {
        let alice = create_signer();
        Role::assign_role<Developer>(&alice, &Developer{});
        Role::assign_role<Developer>(&alice, &Developer{});
    }

    #[test]
    #[expected_failure(abort_code = 5)]
    fun test_revoke_failure() {
        let alice = create_signer();
        Role::revoke_role<Developer>(&alice, &Developer{});
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
