//# init --addresses DD=0x037e26667e58510f6571a0fb964d0fe6
//#                  BadDD=0x7b52d42bd62ccdec1a23350f3f36705f
//#      --private-keys DD=e5127d4e9a9021c7fd4b488388a50e56311ac0af0aefb6442dcf4ef1b838aa15
//#                     BadDD=b1e2f8c73131177ae76c3d69ce59fe82ec0a0c4e0e7cae221e558bc53d97fc3e
//#      --parent-vasps Test Alice

// Test the end-to-end preburn-burn flow

// bad auth: 900660d025a623be5d46adb022781cad

// TODO: consider replacing some of the admin scripts with script function calls


// Register blessed as a preburn entity.

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 @DD x"4829ff62041cd41865517b7dfd438caf" x"" false
//#     -- 0x1::AccountCreationScripts::create_designated_dealer

//# run --signers TreasuryCompliance
//#     --type-args 0x1::XUS::XUS
//#     --args 0 @DD 600 0
//#     -- 0x1::TreasuryComplianceScripts::tiered_mint



// Perform two preburns, one with a value of 55 and the other 45.
//# run --admin-script --signers DiemRoot DD --show-events
script {
    use DiemFramework::XUS::XUS;
    use DiemFramework::Diem;
    use DiemFramework::DiemAccount;

    fun main(_dr: signer, account: signer) {
        let account = &account;
        let old_market_cap = Diem::market_cap<XUS>();
        let with_cap = DiemAccount::extract_withdraw_capability(account);
        // send the coins to the preburn area. market cap should not be affected, but the preburn
        // bucket should increase in size by 100
        DiemAccount::preburn<XUS>(account, &with_cap, 55);
        DiemAccount::preburn<XUS>(account, &with_cap, 45);
        assert!(Diem::market_cap<XUS>() == old_market_cap, 8002);
        assert!(Diem::preburn_value<XUS>() == 100, 8003);
        DiemAccount::pay_from<XUS>(&with_cap, @Alice, 2, x"", x"");
        DiemAccount::restore_withdraw_capability(with_cap);
    }
}



// Cancel the preburn, no matching value found so error.
//
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args @DD 56
//#     -- 0x1::TreasuryComplianceScripts::cancel_burn_with_amount

// Cancel the burn, but a zero value.
//
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args @DD 0
//#     -- 0x1::TreasuryComplianceScripts::cancel_burn_with_amount

// Cancel the mutliple preburns.

//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args @DD 55 --show-events
//#     -- 0x1::TreasuryComplianceScripts::cancel_burn_with_amount

//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args @DD 45 --show-events
//#     -- 0x1::TreasuryComplianceScripts::cancel_burn_with_amount



// Perform a preburn.
//
//# run --admin-script --signers DiemRoot DD --show-events
script {
    use DiemFramework::XUS::XUS;
    use DiemFramework::Diem;
    use DiemFramework::DiemAccount;

    fun main(_dr: signer, account: signer) {
        let account = &account;
        let old_market_cap = Diem::market_cap<XUS>();
        let with_cap = DiemAccount::extract_withdraw_capability(account);
        // send the coins to the preburn area. market cap should not be affected, but the preburn
        // bucket should increase in size by 100
        DiemAccount::preburn<XUS>(account, &with_cap, 100);
        assert!(Diem::market_cap<XUS>() == old_market_cap, 8002);
        assert!(Diem::preburn_value<XUS>() == 100, 8003);
        DiemAccount::restore_withdraw_capability(with_cap);
    }
}

// Second (concurrent) preburn allowed.
//
//# run --signers DD --type-args 0x1::XUS::XUS --args 200 --show-events
//#     -- 0x1::TreasuryComplianceScripts::preburn


// Perform the burn from the blessed account, but wrong value.
// This should fail since there isn't a preburn with a value of 300.
//
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args 0 @DD 300
//#     -- 0x1::TreasuryComplianceScripts::burn_with_amount

// Perform the burn from the blessed account, but a zero value.
// This should fail since the amount is 0.
//
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args 0 @DD 0
//#     -- 0x1::TreasuryComplianceScripts::burn_with_amount

//# run --admin-script --signers DiemRoot TreasuryCompliance --show-events
script {
    use DiemFramework::XUS::XUS;
    use DiemFramework::Diem;

    fun main(_dr: signer, account: signer) {
        let account = &account;
        let old_market_cap = Diem::market_cap<XUS>();
        // do the burn. the market cap should now decrease, and the preburn area should be empty
        Diem::burn<XUS>(account, @DD, 100);
        Diem::burn<XUS>(account, @DD, 200);
        assert!(Diem::market_cap<XUS>() == old_market_cap - 300, 8004);
        assert!(Diem::preburn_value<XUS>() == 0, 8005);
    }
}

// Preburn allowed but larger than balance.
//
//# run --signers DD --type-args 0x1::XUS::XUS --args 501 --show-events
//#     -- 0x1::TreasuryComplianceScripts::preburn


// Try to burn on an account that doesn't have a preburn queue resource.
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args 0 @Alice 0
//#     -- 0x1::TreasuryComplianceScripts::burn_with_amount


// Try to burn on an account that doesn't have a burn capability.
//# run --signers Alice --type-args 0x1::XUS::XUS --args 0 @Alice 0
//#     -- 0x1::TreasuryComplianceScripts::burn_with_amount

// Try to cancel burn on an account that doesn't have a burn capability.
//
//# run --signers Alice --type-args 0x1::XUS::XUS --args @DD 0
//#     -- 0x1::TreasuryComplianceScripts::cancel_burn_with_amount

// Try to preburn to an account that doesn't have a preburn resource
//
//# run --signers Alice --type-args 0x1::XUS::XUS --args 1 --show-events
//#     -- 0x1::TreasuryComplianceScripts::preburn



//# publish
module Test::Holder {
    struct Holder<T> has key {
        a: T,
        b: T,
    }

    public fun hold<T: store>(account: &signer, a: T, b: T) {
        move_to(account, Holder<T>{ a, b})
    }

    public fun get<T: store>(addr: address): (T, T)
    acquires Holder {
        let Holder { a, b} = move_from<Holder<T>>(addr);
        (a, b)
    }
}

//# run --admin-script --signers DiemRoot TreasuryCompliance
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;
    use Test::Holder;

    fun main(_dr: signer, account: signer) {
        let u64_max = 18446744073709551615;
        Holder::hold(
            &account,
            Diem::mint<XUS>(&account, u64_max),
            Diem::mint<XUS>(&account, u64_max)
        );
    }
}

//# run --admin-script --signers DiemRoot DD
script {
    use DiemFramework::Diem::{Self, Diem};
    use DiemFramework::XUS::XUS;
    use Test::Holder;

    fun main(_dr: signer, account: signer) {
        let (xus, coin2) = Holder::get<Diem<XUS>>(@TreasuryCompliance);
        Diem::preburn_to(&account, xus);
        Diem::preburn_to(&account, coin2);
    }
}

//# run --signers DD --type-args 0x1::XUS::XUS --args 1
//#     -- 0x1::TreasuryComplianceScripts::preburn

// Limit exceeded on coin deposit
//
//# run --signers TreasuryCompliance --type-args 0x1::XUS::XUS --args 0 @DD 1
//#     -- 0x1::TreasuryComplianceScripts::burn_with_amount

// //! new-transaction
// script {
// use DiemFramework::Diem;
// use DiemFramework::XUS::XUS;
// fun main(account: signer) {
//     let account = &account;
//     Diem::publish_burn_capability(
//         account,
//         Diem::remove_burn_capability<XUS>(account)
//     );
// }
// }
// // check: "Keep(ABORTED { code: 4,"

//# run --admin-script --signers DiemRoot DD
script {
    use DiemFramework::Diem;
    use DiemFramework::XUS::XUS;
    use Test::Holder;

    fun main(_dr: signer, account: signer) {
        let account = &account;
        let index = 0;
        let max_outstanding_requests = 256;
        let (xus1, xus2) = Holder::get<Diem::Diem<XUS>>(@TreasuryCompliance);
        while (index < max_outstanding_requests) {
            Diem::preburn_to(account, Diem::withdraw(&mut xus1, 1));
            index = index + 1;
        };
        Diem::preburn_to(account, Diem::withdraw(&mut xus1, 1));
        Holder::hold(account, xus1, xus2);
    }
}

// Preburn allowed but amount is zero so aborts.
//
//# run --signers DD --type-args 0x1::XUS::XUS --args 0
//#     -- 0x1::TreasuryComplianceScripts::preburn
