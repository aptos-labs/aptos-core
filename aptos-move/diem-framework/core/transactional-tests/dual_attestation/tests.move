//# init --parent-vasps Alice Bob --validators FreddyMac

//# run --admin-script --signers DiemRoot FreddyMac
script{
    use DiemFramework::DualAttestation;

    fun main() {
        DualAttestation::get_cur_microdiem_limit();
    }
}

//# run --admin-script --signers DiemRoot Alice
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, not_blessed: signer) {
        DualAttestation::set_microdiem_limit(&not_blessed, 99);
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, blessed: signer) {
        DualAttestation::set_microdiem_limit(&blessed, 1001);
    }
}

// TODO: DualAttestation::publish_credential is now a friend function
// Make this into a unit test.
// //! new-transaction
// //! sender: blessed
// script{
//     use DiemFramework::DualAttestation;
//     fun main(account: signer) {
//     let account = &account;
//         DualAttestation::publish_credential(account, account, x"");
//     }
// }
// // check: "Keep(ABORTED { code: 1283,"

// TODO: Make into unit test
// //! new-transaction
// //! sender: blessed
// script{
//     use DiemFramework::DualAttestation;
//     fun main(account: signer) {
//     let account = &account;
//         DualAttestation::publish_credential(account, account, x"");
//     }
// }
// // check: "Keep(ABORTED { code: 1283,"

// TODO: Make into unit test
// //! new-transaction
// //! sender: bob
// script{
//     use DiemFramework::DualAttestation;
//     fun main(account: signer) {
//     let account = &account;
//         DualAttestation::publish_credential(account, account, x"");
//     }
// }
// // check: "Keep(ABORTED { code: 258,"

//# run --admin-script --signers DiemRoot TreasuryCompliance
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, account: signer) {
        DualAttestation::rotate_base_url(&account, x"");
    }
}

//# run --admin-script --signers DiemRoot Bob --show-events
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, account: signer) {
        DualAttestation::rotate_base_url(&account, x"");
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, account: signer) {
        DualAttestation::rotate_compliance_public_key(&account, x"");
    }
}

//# run --admin-script --signers DiemRoot Bob
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, account: signer) {
        DualAttestation::rotate_compliance_public_key(&account, x"");
    }
}

// TODO: why are the two following transactions identical?
//
//# run --admin-script --signers DiemRoot Bob
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, account: signer) {
        DualAttestation::initialize(&account);
    }
}

//# run --admin-script --signers DiemRoot Bob
script{
    use DiemFramework::DualAttestation;

    fun main(_dr: signer, account: signer) {
        DualAttestation::initialize(&account);
    }
}

// TODO: Make into unit test
// //! new-transaction
// //! sender: diemroot
// //! execute-as: freddymac
// script{
// use DiemFramework::DualAttestation;
// fun main(dr_account: signer, freddy: signer) {
//     let dr_account = &dr_account;
//     let freddy = &freddy;
//     DualAttestation::publish_credential(freddy, dr_account, b"freddy");
//     DualAttestation::publish_credential(freddy, dr_account, b"freddy");
// }
// }
// // check: "Discard(INVALID_WRITE_SET)"

//# run --admin-script --signers DiemRoot DiemRoot
script{
    use DiemFramework::DualAttestation;

    fun main() {
        DualAttestation::human_name(@FreddyMac);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script{
    use DiemFramework::DualAttestation;

    fun main() {
        DualAttestation::base_url(@FreddyMac);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script{
    use DiemFramework::DualAttestation;

    fun main() {
        DualAttestation::compliance_public_key(@FreddyMac);
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script{
    use DiemFramework::DualAttestation;

    fun main() {
        DualAttestation::expiration_date(@FreddyMac);
    }
}
