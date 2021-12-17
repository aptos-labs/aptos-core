//# init --parent-vasps Test Bob
//#      --addresses Vasp=0xeacc26ce1e89961ca6393ab68fbf299a
//#                  Child=0xf1e113a94a4088ea7d8f41b6cb21039e
//#                  Alice=0x09540974260394cb78415f6fb6413595
//#      --private-keys Vasp=b03037df306fce3898c566bded861b89403e6b87fd6099c6bb70122e8508786a
//#                     Child=a5892658fd71aee083e448d7c8f396bb7c7b8589768471ef82863662f3453cae
//#                     Alice=6e642294e183faa7721b35d6e056494cbfe946e6da998b3024dea9f2b9d57752

// TODO: switch to script function calls?

// Keep these tests until adding unit tests for the prologue and Diem Account around freezing
// We need to keep some of them to test for events as well

//# publish
module DiemRoot::Test {
    public(script) fun nop() {}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script {
use DiemFramework::AccountFreezing;
// Make sure we can freeze and unfreeze accounts.
fun main(_dr: signer, account: signer) {
    AccountFreezing::freeze_account(&account, @Bob);
}
}

//# run --signers Bob -- 0xA550C18::Test::nop

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script {
use DiemFramework::AccountFreezing::{Self};
fun main(_dr: signer, account: signer) {
    AccountFreezing::unfreeze_account(&account, @Bob);
}
}

//# run --signers Bob -- 0xA550C18::Test::nop

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script {
use DiemFramework::AccountFreezing::{Self};
fun main(_dr: signer, account: signer) {
    AccountFreezing::freeze_account(&account, @DiemRoot);
}
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 @Vasp x"8ae64fead0bd7f052f0b608e8e704960" b"vasp" true
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# run --signers Vasp
//#     --type-args 0x1::XUS::XUS
//#     --args @Child x"d803a42f5d154ce1f2bdefbf2630ce5a" true 0
//#     -- 0x1::AccountCreationScripts::create_child_vasp_account

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script {
use DiemFramework::AccountFreezing;
// Freezing a child account doesn't freeze the root, freezing the root
// doesn't freeze the child
fun main(_dr: signer, account: signer) {
    AccountFreezing::freeze_account(&account, @Child);
    assert!(AccountFreezing::account_is_frozen(@Child), 3);
    assert!(!AccountFreezing::account_is_frozen(@Vasp), 4);
    AccountFreezing::unfreeze_account(&account, @Child);
    assert!(!AccountFreezing::account_is_frozen(@Child), 5);
    AccountFreezing::freeze_account(&account, @Vasp);
    assert!(AccountFreezing::account_is_frozen(@Vasp), 6);
    assert!(!AccountFreezing::account_is_frozen(@Child), 7);
    AccountFreezing::unfreeze_account(&account, @Vasp);
    assert!(!AccountFreezing::account_is_frozen(@Vasp), 8);
}
}

//# publish
module Test::Holder {
    struct Holder<T> has key { x: T }
    public fun hold<T: store>(account: &signer, x: T) {
        move_to(account, Holder<T>{ x })
    }

    public fun get<T: store>(addr: address): T
    acquires Holder {
       let Holder<T> { x } = move_from<Holder<T>>(addr);
       x
    }
}

//# run --admin-script --signers DiemRoot Vasp
script {
use DiemFramework::DiemAccount;
use Test::Holder;
fun main(_dr: signer, account: signer) {
    let cap = DiemAccount::extract_withdraw_capability(&account);
    Holder::hold(&account, cap);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::AccountFreezing;
    fun main(_dr: signer, account: signer) {
        AccountFreezing::freeze_account(&account, @Vasp);
        assert!(AccountFreezing::account_is_frozen(@Vasp), 1);
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::AccountFreezing;
    fun main(_dr: signer, account: signer) {
        AccountFreezing::freeze_account(&account, @Vasp);
        assert!(AccountFreezing::account_is_frozen(@Vasp), 1);
    }
}

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0
//#            @Alice
//#            x"3aa76fbc2fdf6fb765ef48484c7357f5"
//#            b"alice"
//#            true
//#     -- 0x1::AccountCreationScripts::create_parent_vasp_account

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use Test::Holder;
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;
    fun main(_dr: signer, account: signer) {
        let cap = Holder::get<DiemAccount::WithdrawCapability>(@Vasp);
        DiemAccount::pay_from<XUS>(&cap, @Alice, 0, x"", x"");
        Holder::hold(&account, cap);
    }
}

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;
    fun main(_dr: signer, account: signer) {
        let cap = DiemAccount::extract_withdraw_capability(&account);
        DiemAccount::pay_from<XUS>(&cap, @Vasp, 0, x"", x"");
        DiemAccount::restore_withdraw_capability(cap);
    }
}

//# run --admin-script --signers DiemRoot Alice
script {
    use DiemFramework::DiemAccount;
    use DiemFramework::XUS::XUS;
    fun main(_dr: signer, account: signer) {
        let cap = DiemAccount::extract_withdraw_capability(&account);
        DiemAccount::pay_from<XUS>(&cap, @Vasp, 0, x"", x"");
        DiemAccount::restore_withdraw_capability(cap);
    }
}

// TODO: make into unit test
// //! new-transaction
// //! sender: alice
// script {
// use DiemFramework::AccountFreezing;
// fun main(_dr: signer, account: signer) {
//     let account = &account;
//     AccountFreezing::create(account);
// }
// }
// // check: "Keep(ABORTED { code: 518,"

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::AccountFreezing;
fun main(_dr: signer, account: signer) {
    AccountFreezing::freeze_account(&account, @0x0);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::AccountFreezing;
fun main(_dr: signer, account: signer) {
    AccountFreezing::unfreeze_account(&account, @0x0);
}
}
