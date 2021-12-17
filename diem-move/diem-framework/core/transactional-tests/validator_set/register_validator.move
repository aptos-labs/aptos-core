//# init --validators Vivian Viola --parent-vasps Alice

// Check that the validator account config works.
//
//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemSystem;
    use Std::Signer;

    fun main(_dr: signer, account: signer) {
        let sender = Signer::address_of(&account);
        assert!(!DiemSystem::is_validator(sender), 1);
        assert!(!DiemSystem::is_validator(@Alice), 2);
        assert!(DiemSystem::is_validator(@Vivian), 3);
        assert!(DiemSystem::is_validator(@Viola), 4);
        // number of validators should equal the number we declared
        // TODO: currently 10 test validators are part of the test genesis so we have to count them.
        assert!(DiemSystem::validator_set_size() == 10 + 2, 5);
        assert!(DiemSystem::get_ith_validator_address(10) == @Vivian, 6);
        assert!(DiemSystem::get_ith_validator_address(11) == @Viola, 7);
    }
}

//# run --admin-script --signers DiemRoot Vivian --show-events
script{
    use DiemFramework::DiemSystem;
    use Std::Signer;

    // check that sending from validator accounts works
    fun main(_dr: signer, account: signer) {
        let sender = Signer::address_of(&account);
        assert!(DiemSystem::is_validator(sender), 8);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot --show-events
script{
    use DiemFramework::DiemAccount;

    // register Alice as a validator candidate
    fun main(_dr: signer, creator: signer) {
        DiemAccount::create_validator_account(
            &creator, @0xAA, x"00000000000000000000000000000000", b"owner_name"
        );
    }
}

// TODO(valerini): enable the following test once the sender format is supported
// //! new-transaction
// //! sender: 0xAA
// script{

//     // register Alice as a validator candidate, then rotate a key + check that it worked.
//     fun main(account: signer) {
//         // Alice registers as a validator candidate

//         // Rotating the consensus_pubkey should work

//         // Rotating the validator's full config
//     }
// }

// // check: "Keep(EXECUTED)"
