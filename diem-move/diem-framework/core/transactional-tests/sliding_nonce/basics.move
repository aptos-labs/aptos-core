//# init --parent-vasps Alice Bob

// ****
// Account setup - bob is account with nonce resource and alice is a regular account
// ****

// TODO: SlidingNonce::publish(account) is now a friend function
// Sliding nonces are publish at an address iff the account is Diem Root
// or Treasury Compliance, so we don't want sliding nonces to be published
// at other addresses. Unfortunately, most of this code depends on that.
// These should probably be unit tests.

// Make into unit tests
// //! new-transaction
// //! sender: bob
// script {
//     use DiemFramework::SlidingNonce;

//     fun main(account: signer) {
//     let account = &account;
//         SlidingNonce::publish(account);
//         SlidingNonce::record_nonce_or_abort(account, 129);
//     }
// }

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::SlidingNonce;

    fun main(_dr: signer, account: signer) {
        SlidingNonce::record_nonce_or_abort(&account, 1);
    }
}

// //! new-transaction
// //! sender: bob
// script {
//     use DiemFramework::SlidingNonce;
//     fun main(account: signer) {
//     let account = &account;
//         SlidingNonce::publish(account);
//     }
// }

//# run --admin-script --signers DiemRoot Bob
script {
    use DiemFramework::SlidingNonce;
    fun main(_dr: signer, account: signer) {
        SlidingNonce::try_record_nonce(&account, 1);
    }
}

// //! new-transaction
// script {
//     use DiemFramework::SlidingNonce;
//     fun main(account: signer) {
//     let account = &account;
//         SlidingNonce::publish(account);
//     }
// }

// //! new-transaction
// script {
//     use DiemFramework::SlidingNonce;
//     fun main(account: signer) {
//     let account = &account;
//         SlidingNonce::publish(account);
//     }
// }

// //! new-transaction
// script {
//     use DiemFramework::SlidingNonce;
//     fun main(default_account: signer) {
//     let default_account = &default_account;
//         SlidingNonce::publish(default_account);
//     }
// }

// //! new-transaction
// script {
//     use DiemFramework::SlidingNonce;
//     fun main(account: signer) {
//     let account = &account;
//         SlidingNonce::publish(account);
//     }
// }

// //! new-transaction
// script {
//     use DiemFramework::SlidingNonce;
//     fun main(account: signer) {
//     let account = &account;
//         SlidingNonce::publish(account);
//     }
// }

// //! new-transaction
// script {
//     use DiemFramework::SlidingNonce;
//     fun main(default_account: signer) {
//     let default_account = &default_account;
//         SlidingNonce::publish(default_account);
//     }
// }
