// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Support for encoding transactions for common situations.

use crate::account::Account;
use aptos_transaction_builder::aptos_stdlib::{
    encode_create_account_script_function, encode_rotate_authentication_key_script_function,
    encode_transfer_script_function,
};
use aptos_types::{
    account_config,
    transaction::{RawTransaction, Script, SignedTransaction, TransactionArgument},
};
use move_ir_compiler::Compiler;
use once_cell::sync::Lazy;

pub static EMPTY_SCRIPT: Lazy<Vec<u8>> = Lazy::new(|| {
    let code = "
    main(account: signer) {
    label b0:
      return;
    }
";

    let compiler = Compiler {
        deps: cached_framework_packages::modules().iter().collect(),
    };
    compiler.into_script_blob(code).expect("Failed to compile")
});

pub static MULTI_AGENT_SWAP_SCRIPT: Lazy<Vec<u8>> = Lazy::new(|| {
    let code = "
    import 0x1.DiemAccount;
    import 0x1.Signer;
    import 0x1.XDX;
    import 0x1.XUS;

    // Alice and Bob agree on the value of amount_xus and amount_xdx off-chain.
    main(alice: signer, bob: signer, amount_xus: u64, amount_xdx: u64) {
        // First, Alice pays Bob in currency XUS.
        let alice_withdrawal_cap: DiemAccount.WithdrawCapability;
        let bob_withdrawal_cap: DiemAccount.WithdrawCapability;
        let alice_addr: address;
        let bob_addr: address;
    label b0:
        alice_withdrawal_cap = DiemAccount.extract_withdraw_capability(&alice);
        bob_addr = Signer.address_of(&bob);
        DiemAccount.pay_from<XUS.XUS>(
            &alice_withdrawal_cap, move(bob_addr), move(amount_xus), h\"\", h\"\"
        );
        DiemAccount.restore_withdraw_capability(move(alice_withdrawal_cap));

        // Then, Bob pays Alice in currency XDX.
        bob_withdrawal_cap = DiemAccount.extract_withdraw_capability(&bob);
        alice_addr = Signer.address_of(&alice);
        DiemAccount.pay_from<XDX.XDX>(
            &bob_withdrawal_cap, move(alice_addr), move(amount_xdx), h\"\", h\"\"
        );
        DiemAccount.restore_withdraw_capability(move(bob_withdrawal_cap));
        return;
    }
";

    let compiler = Compiler {
        deps: cached_framework_packages::modules().iter().collect(),
    };
    compiler.into_script_blob(code).expect("Failed to compile")
});

pub static MULTI_AGENT_MINT_SCRIPT: Lazy<Vec<u8>> = Lazy::new(|| {
    let code = "
    import 0x1.DiemAccount;
    import 0x1.Signer;
    import 0x1.XDX;
    import 0x1.XUS;

    main<CoinType>(
        tc_account: signer,
        dd_account: signer,
        vasp_account: signer,
        amount: u64,
        tier_index: u64
    ) {

        let dd_address: address;
        let dd_withdrawal_cap: DiemAccount.WithdrawCapability;
        let vasp_address: address;
    label b0:
        dd_address = Signer.address_of(&dd_account);
        // First, TC mints to DD.
        DiemAccount.tiered_mint<CoinType>(
            &tc_account, move(dd_address), copy(amount), move(tier_index)
        );

        // Then, DD distributes funds to VASP.
        dd_withdrawal_cap = DiemAccount.extract_withdraw_capability(&dd_account);
        vasp_address = Signer.address_of(&vasp_account);
        DiemAccount.pay_from<CoinType>(
            &dd_withdrawal_cap, move(vasp_address), move(amount), h\"\", h\"\"
        );
        DiemAccount.restore_withdraw_capability(move(dd_withdrawal_cap));
        return;
    }
";

    let compiler = Compiler {
        deps: cached_framework_packages::modules().iter().collect(),
    };
    compiler.into_script_blob(code).expect("Failed to compile")
});

pub fn empty_txn(
    sender: &Account,
    seq_num: u64,
    max_gas_amount: u64,
    gas_unit_price: u64,
) -> SignedTransaction {
    sender
        .transaction()
        .script(Script::new(EMPTY_SCRIPT.to_vec(), vec![], vec![]))
        .sequence_number(seq_num)
        .max_gas_amount(max_gas_amount)
        .gas_unit_price(gas_unit_price)
        .sign()
}

/// Returns a transaction to create a new account with the given arguments.
pub fn create_account_txn(
    sender: &Account,
    new_account: &Account,
    seq_num: u64,
) -> SignedTransaction {
    sender
        .transaction()
        .payload(encode_create_account_script_function(
            *new_account.address(),
        ))
        .sequence_number(seq_num)
        .sign()
}

/// Returns a transaction to transfer coin from one account to another (possibly new) one, with the
/// given arguments.
pub fn peer_to_peer_txn(
    sender: &Account,
    receiver: &Account,
    seq_num: u64,
    transfer_amount: u64,
) -> SignedTransaction {
    // get a SignedTransaction
    sender
        .transaction()
        .payload(encode_transfer_script_function(
            *receiver.address(),
            transfer_amount,
        ))
        .sequence_number(seq_num)
        .sign()
}

/// Returns a transaction to change the keys for the given account.
pub fn rotate_key_txn(sender: &Account, new_key_hash: Vec<u8>, seq_num: u64) -> SignedTransaction {
    sender
        .transaction()
        .payload(encode_rotate_authentication_key_script_function(
            new_key_hash,
        ))
        .sequence_number(seq_num)
        .sign()
}

/// Returns a transaction to change the keys for the given account.
pub fn raw_rotate_key_txn(sender: &Account, new_key_hash: Vec<u8>, seq_num: u64) -> RawTransaction {
    sender
        .transaction()
        .payload(encode_rotate_authentication_key_script_function(
            new_key_hash,
        ))
        .sequence_number(seq_num)
        .raw()
}

/// Returns a transaction to swap currencies between two accounts.
pub fn multi_agent_swap_txn(
    sender: &Account,
    secondary_signer: &Account,
    seq_num: u64,
    xus_amount: u64,
    xdx_amount: u64,
) -> SignedTransaction {
    let args: Vec<TransactionArgument> = vec![
        TransactionArgument::U64(xus_amount),
        TransactionArgument::U64(xdx_amount),
    ];

    // get a SignedTransaction
    sender
        .transaction()
        .secondary_signers(vec![secondary_signer.clone()])
        .script(Script::new(MULTI_AGENT_SWAP_SCRIPT.to_vec(), vec![], args))
        .sequence_number(seq_num)
        .sign_multi_agent()
}

/// Returns a multi-agent p2p transaction.
/// Returns a transaction to mint coins from TC to DD to VASP.
pub fn multi_agent_mint_txn(
    tc_account: &Account,
    dd_account: &Account,
    vasp_account: &Account,
    seq_num: u64,
    amount: u64,
    tier_index: u64,
) -> SignedTransaction {
    let args: Vec<TransactionArgument> = vec![
        TransactionArgument::U64(amount),
        TransactionArgument::U64(tier_index),
    ];
    // get a SignedTransaction
    tc_account
        .transaction()
        .secondary_signers(vec![dd_account.clone(), vasp_account.clone()])
        .script(Script::new(
            MULTI_AGENT_MINT_SCRIPT.to_vec(),
            vec![account_config::xus_tag()],
            args,
        ))
        .sequence_number(seq_num)
        .sign_multi_agent()
}

/// Returns an unsigned raw transaction to swap currencies between two accounts.
pub fn raw_multi_agent_swap_txn(
    sender: &Account,
    secondary_signer: &Account,
    seq_num: u64,
    xus_amount: u64,
    xdx_amount: u64,
) -> RawTransaction {
    let args: Vec<TransactionArgument> = vec![
        TransactionArgument::U64(xus_amount),
        TransactionArgument::U64(xdx_amount),
    ];

    sender
        .transaction()
        .secondary_signers(vec![secondary_signer.clone()])
        .script(Script::new(MULTI_AGENT_SWAP_SCRIPT.to_vec(), vec![], args))
        .sequence_number(seq_num)
        .raw()
}

pub fn multi_agent_swap_script(xus_amount: u64, xdx_amount: u64) -> Script {
    let args: Vec<TransactionArgument> = vec![
        TransactionArgument::U64(xus_amount),
        TransactionArgument::U64(xdx_amount),
    ];
    Script::new(MULTI_AGENT_SWAP_SCRIPT.to_vec(), vec![], args)
}

pub fn multi_agent_mint_script(mint_amount: u64, tier_index: u64) -> Script {
    let args: Vec<TransactionArgument> = vec![
        TransactionArgument::U64(mint_amount),
        TransactionArgument::U64(tier_index),
    ];
    Script::new(MULTI_AGENT_MINT_SCRIPT.to_vec(), vec![], args)
}
