//# init --addresses Parent=0x6dacafcd926e5bface48cade21c9de2d
//#      --private-keys Parent=8dd834d205b2e4901e9f9719fbe80b4323127892658d9b269575309179a41f88
//#      --validators Vivian

// TODO: is this a duplicate of create_parent_and_child_vasp_accounts.move?

// Create a parent vasp.
//
//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0
//#            @Parent
//#            x"67f819e80aef87be6bbbb30fd671a52f"
//#            b"Parent"
//#            false
//#     --show-events
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account


//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DiemTimestamp;
    use DiemFramework::VASP;
    use DiemFramework::DualAttestation;

    fun main() {
        assert!(VASP::is_vasp(@Parent), 2001);
        assert!(VASP::is_parent(@Parent), 2002);
        assert!(!VASP::is_child(@Parent), 2003);

        assert!(VASP::parent_address(@Parent) == @Parent, 2005);
        assert!(DualAttestation::compliance_public_key(@Parent) == x"", 2006);
        assert!(DualAttestation::human_name(@Parent) == b"Parent", 2007);
        assert!(DualAttestation::base_url(@Parent) == x"", 2008);
        assert!(
            DualAttestation::expiration_date(@Parent) > DiemTimestamp::now_microseconds(),
            2009
        );
        assert!(VASP::num_children(@Parent) == 0, 2010);
    }
}


// Create the first child vasp account with dummy address + dummy auth key prefix.
//
//# run --signers Parent
//#     --type-args 0x1::XUS::XUS
//#     --args 0xAA
//#            x"00000000000000000000000000000000"
//#            false
//#            0
//#     --show-events
//#     -- 0x1::AccountCreationScripts::create_child_vasp_account


//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::VASP;

    fun main() {
        assert!(VASP::num_children(@Parent) == 1, 2011);
        assert!(VASP::parent_address(@0xAA) == @Parent, 2012);
    }
}


// Create the second child vasp account with dummy address + dummy auth key prefix.
//
//# run --signers Parent
//#     --type-args 0x1::XUS::XUS
//#     --args 0xBB
//#            x"00000000000000000000000000000000"
//#            false
//#            0
//#     --show-events
//#     -- 0x1::AccountCreationScripts::create_child_vasp_account


//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::VASP;

    fun main() {
        assert!(VASP::num_children(@Parent) == 2, 2013);
        assert!(VASP::parent_address(@0xBB) == @Parent, 2014);
    }
}


// TODO: consider splitting the tests into multiple files?

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DualAttestation;

    fun main() {
        let old_pubkey = DualAttestation::compliance_public_key(@Parent);
        let new_pubkey = x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
        assert!(old_pubkey != new_pubkey, 2015);
    }
}


// Rotate the dual attestation info.
//
//# run --signers Parent
//#     --args x"" x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"
//#     --show-events
//#     -- 0x1::AccountAdministrationScripts::rotate_dual_attestation_info


//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::DualAttestation;

    fun main() {
        let old_pubkey = DualAttestation::compliance_public_key(@Parent);
        let new_pubkey = x"3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
        assert!(old_pubkey == new_pubkey, 2016);
    }
}



// Getting the parent VASP address of a non-VASP should abort.
//
//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::VASP;

    fun main() {
        VASP::parent_address(@Vivian);
    }
}


// TODO: VASP::publish_parent_vasp_credential is now a friend function
// Make into unit test.
// //! new-transaction
// //! sender: blessed
// script {
// use DiemFramework::VASP;
// fun main(account: signer) {
//     let account = &account;
//     VASP::publish_parent_vasp_credential(account, account);
//     abort 99
// }
// }
// // check: "Keep(ABORTED { code: 771,"

// //! new-transaction
// //! sender: diemroot
// script {
// use DiemFramework::VASP;
// fun main(account: signer) {
//     let account = &account;
//     VASP::publish_parent_vasp_credential(account, account);
// }
// }
// // check: "Keep(ABORTED { code: 258,"

// //! new-transaction
// //! sender: blessed
// script {
// use DiemFramework::VASP;
// fun main(account: signer) {
//     let account = &account;
//     VASP::publish_child_vasp_credential(account, account);
// }
// }
// // check: "Keep(ABORTED { code: 771,"

// //! new-transaction
// //! sender: blessed
// script {
// use DiemFramework::VASP;
// fun main(account: signer) {
//     let account = &account;
//     VASP::publish_child_vasp_credential(account, account);
// }
// }
// // check: "Keep(ABORTED { code: 771,"

// //! new-transaction
// //! sender: parent
// script {
// use DiemFramework::VASP;
// fun main(account: signer) {
//     let account = &account;
//     VASP::publish_child_vasp_credential(account, account);
// }
// }
// // check: "Keep(ABORTED { code: 2307,"


//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::VASP;

    fun main() {
        assert!(!VASP::is_same_vasp(@Parent, @TreasuryCompliance), 2017);
        assert!(!VASP::is_same_vasp(@TreasuryCompliance, @Parent), 2018);
    }
}
