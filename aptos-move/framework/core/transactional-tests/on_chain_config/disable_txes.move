//# init --validators Vivian --parent-vasps Alice

//# block --proposer Vivian --time 2

// Disable txes from all accounts except DiemRoot.
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemTransactionPublishingOption;
fun main(dr: signer, _dr2: signer) {
    DiemTransactionPublishingOption::halt_all_transactions(&dr);
}
}
// Sending allowlisted script from normal account fails
//# run --signers Alice -- 0x1::AccountAdministrationScripts::rotate_authentication_key

// TODO: module publishing doesn't seem to be halted. Is this intentional?
// publish
// module Alice::M {}

//# block --proposer Vivian --time 3


// Re-enable. this also tests that sending from DiemRoot succeeds.
//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::DiemTransactionPublishingOption;
fun main(dr: signer, _dr2: signer) {
    DiemTransactionPublishingOption::resume_transactions(&dr);
}
}

// Sending from normal account succeeds again.
// Note: the transaction will still abort due to the bad key supplied. This is normal.
//# run --signers Alice --args x"" -- 0x1::AccountAdministrationScripts::rotate_authentication_key

// A normal account has insufficient privs to halt transactions.
//# run --admin-script --signers DiemRoot Vivian
script {
use DiemFramework::DiemTransactionPublishingOption;
fun main(dr: signer, vv: signer) {
    DiemTransactionPublishingOption::halt_all_transactions(&vv);
}
}
