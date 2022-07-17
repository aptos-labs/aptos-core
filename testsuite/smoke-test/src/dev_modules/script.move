// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Note: If this test file fails to run, it is possible that the
// compiled version of the Move stdlib needs to be updated. This code
// is compiled with the latest compiler and stdlib, but it runs with
// the compiled stdlib.

script {
use aptos_framework::Aptos;
use aptos_framework::AptosAccount;
use aptos_framework::XUS::XUS;
use {{sender}}::MyModule;

fun main(account: signer, recipient: address, amount: u64) {
    let with_cap = Aptosaccount::extract_withdraw_capability(&account);
    Aptosaccount::pay_from<XUS>(&with_cap, recipient, amount, x"", x"");
    Aptosaccount::restore_withdraw_capability(with_cap);
    let coin = MyModule::id<XUS>(Aptos::zero<XUS>());
    Aptos::destroy_zero(coin)
}
}
