// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Note: If this test file fails to run, it is possible that the
// compiled version of the Move stdlib needs to be updated. This code
// is compiled with the latest compiler and stdlib, but it runs with
// the compiled stdlib.

script {
use AptosFramework::Aptos;
use AptosFramework::AptosAccount;
use AptosFramework::XUS::XUS;
use {{sender}}::MyModule;

fun main(account: signer, recipient: address, amount: u64) {
    let with_cap = AptosAccount::extract_withdraw_capability(&account);
    AptosAccount::pay_from<XUS>(&with_cap, recipient, amount, x"", x"");
    AptosAccount::restore_withdraw_capability(with_cap);
    let coin = MyModule::id<XUS>(Aptos::zero<XUS>());
    Aptos::destroy_zero(coin)
}
}
