//# init --parent-vasps Test Bob

// TODO: consider converting some of these into unit tests.

//# publish
module Test::Holder {
    struct Holder<T> has key { x: T }
    public fun hold<T: store>(account: &signer, x: T)  {
        move_to(account, Holder<T> { x })
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::Diem;
use DiemFramework::XUS::XUS;
use Test::Holder;
fun main(_dr: signer, account: signer) {
    let account = &account;
    let xus = Diem::mint<XUS>(account, 10000);
    assert!(Diem::value<XUS>(&xus) == 10000, 0);

    let (xus1, xus2) = Diem::split(xus, 5000);
    assert!(Diem::value<XUS>(&xus1) == 5000 , 0);
    assert!(Diem::value<XUS>(&xus2) == 5000 , 2);
    let tmp = Diem::withdraw(&mut xus1, 1000);
    assert!(Diem::value<XUS>(&xus1) == 4000 , 4);
    assert!(Diem::value<XUS>(&tmp) == 1000 , 5);
    Diem::deposit(&mut xus1, tmp);
    assert!(Diem::value<XUS>(&xus1) == 5000 , 6);
    let xus = Diem::join(xus1, xus2);
    assert!(Diem::value<XUS>(&xus) == 10000, 7);
    Holder::hold(account, xus);

    Diem::destroy_zero(Diem::zero<XUS>());
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::Diem;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    let account = &account;
    Diem::destroy_zero(Diem::mint<XUS>(account, 1));
}
}

// TODO: this was a converted test with its original semantics preserved.
// However, it's not clear to me what its intention is.
//
//# publish
module DiemRoot::Helper {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;


    public(script) fun run() {
        let coins = Diem::zero<XUS>();
        Diem::approx_xdx_for_coin<XUS>(&coins);
        Diem::destroy_zero(coins);
    }
}
//# run --signers Bob --gas-currency XUS -- 0xA550C18::Helper::run

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    fun main()  {
        Diem::destroy_zero(
            Diem::zero<u64>()
        );
    }
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
    use DiemFramework::Diem;
    use DiemFramework::XDX::XDX;
    use DiemFramework::XUS::XUS;
    fun main()  {
        assert!(!Diem::is_synthetic_currency<XUS>(), 9);
        assert!(Diem::is_synthetic_currency<XDX>(), 10);
        assert!(!Diem::is_synthetic_currency<u64>(), 11);
    }
}

// TODO: this was commented out in the original functional test.
//
// //! new-transaction
// //! sender: blessed
// script {
//     use DiemFramework::Diem;
//     use DiemFramework::XUS::XUS;
//     use Test::Holder;
//     fun main(account: signer)  {
//     let account = &account;
//         Holder::hold(
//             account,
//             Diem::remove_burn_capability<XUS>(account)
//         );
//     }
// }
// // check: "Keep(EXECUTED)"

// //! new-transaction
// //! sender: diemroot
// script {
// use DiemFramework::Diem;
// use Std::FixedPoint32;
// use Test::Holder;
// fun main(account: signer) {
//     let account = &account;
//     let (mint_cap, burn_cap) = Diem::register_currency<u64>(
//         account, FixedPoint32::create_from_rational(1, 1), true, 10, 10, b"wat"
//     );
//     Diem::publish_burn_capability(account, burn_cap);
//     Holder::hold(account, mint_cap);
// }
// }
// // check: "Keep(ABORTED { code: 258,"

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::Diem;
use Std::FixedPoint32;
use Test::Holder;
fun main(_dr: signer, account: signer) {
    let account = &account;
    let (mint_cap, burn_cap) = Diem::register_currency<u64>(
        account, FixedPoint32::create_from_rational(1, 1), true, 10, 10, b"wat"
    );
    Holder::hold(account, mint_cap);
    Holder::hold(account, burn_cap);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::Diem;
use Std::FixedPoint32;
fun main(_dr: signer, account: signer) {
    let account = &account;
    Diem::register_SCS_currency<u64>(
        account, account, FixedPoint32::create_from_rational(1, 1), 10, 10, b"wat"
    );
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::Diem;
use DiemFramework::XUS::XUS;
use Test::Holder;
fun main(_dr: signer, account: signer) {
    let account = &account;
    Holder::hold(account, Diem::create_preburn<XUS>(account));
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::Diem;
use DiemFramework::XDX::XDX;
fun main(_dr: signer, account: signer) {
    let account = &account;
    Diem::publish_preburn_queue_to_account_for_test<XDX>(account, account);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::Diem;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    let account = &account;
    Diem::publish_preburn_queue_to_account_for_test<XUS>(account, account);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::Diem;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, account: signer) {
    let account = &account;
    let xus = Diem::mint<XUS>(account, 1);
    let tmp = Diem::withdraw(&mut xus, 10);
    Diem::destroy_zero(tmp);
    Diem::destroy_zero(xus);
}
}

//# run --admin-script --signers DiemRoot DiemRoot
script {
use DiemFramework::Diem;
use DiemFramework::XUS::XUS;
use DiemFramework::XDX::XDX;
fun main() {
    assert!(Diem::is_SCS_currency<XUS>(), 99);
    assert!(!Diem::is_SCS_currency<XDX>(), 98);
    assert!(!Diem::is_synthetic_currency<XUS>(), 97);
    assert!(Diem::is_synthetic_currency<XDX>(), 96);
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::CoreAddresses;
fun main(_dr: signer, account: signer) {
    let account = &account;
    CoreAddresses::assert_currency_info(account)
}
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
use DiemFramework::Diem;
use DiemFramework::XUS::XUS;
fun main(_dr: signer, tc_account: signer) {
    let tc_account = &tc_account;
    let max_u64 = 18446744073709551615;
    let coin1 = Diem::mint<XUS>(tc_account, max_u64);
    let coin2 = Diem::mint<XUS>(tc_account, 1);
    Diem::deposit(&mut coin1, coin2);
    Diem::destroy_zero(coin1);
}
}
