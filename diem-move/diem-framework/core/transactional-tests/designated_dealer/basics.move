//# init --validators Vivian
//#      --parent-vasps Alice
//#      --addresses Dave=0xf42400810cda384c1966c472bfab11f7
//#      --private-keys Dave=f51472493bac725c7284a12c56df41aa3475d731ec289015782b0b9c741b24b5

// TODO: Commented out because DesignatedDealer::publish_designated_dealer_credential
// is now a friend, so not accessible.  Keeping the code because it will soon become
// a unit test.
// //! new-transaction
// script {
// use DiemFramework::DesignatedDealer;
// use DiemFramework::XUS::XUS;
// fun main(account: signer) {
//     let account = &account;
//     DesignatedDealer::publish_designated_dealer_credential<XUS>(
//         account, account, false
//     );
// }
// }
// // check: "Keep(ABORTED { code: 258,"

// TODO: friend function problem
// //! new-transaction
// //! sender: blessed
// script {
// use DiemFramework::DesignatedDealer;
// use DiemFramework::XUS::XUS;
// fun main(account: signer) {
//     let account = &account;
//     DesignatedDealer::publish_designated_dealer_credential<XUS>(
//         account, account, false
//     );
// }
// }
// // check: "Keep(ABORTED { code: 1539,"

//# run --admin-script --signers DiemRoot Alice
script {
use DiemFramework::DesignatedDealer;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DesignatedDealer::add_currency_for_test<XUS>(&account, &account);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::DesignatedDealer;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    DesignatedDealer::add_currency_for_test<XUS>(&account, &account);
}
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 @Dave x"4f52c9f095d4e46c0110c7360ae378a8" x"" false
//#     --show-events
//#     -- 0x1::AccountCreationScripts::create_designated_dealer

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 @Dave 1000000 0
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint

//# block --proposer Vivian --time 95000000000

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 @Dave 1000000 0
//#     --expiration 95000000001
//#     --show-events
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint
