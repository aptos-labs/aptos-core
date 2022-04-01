// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::AccountAddress;
use framework::{encode_transfer_script_function, ScriptFunctionCall};

fn demo_p2p_script_function() {
    let payee = AccountAddress([
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
        0x22, 0x22,
    ]);
    let amount = 1234567;

    // Now encode and decode a peer to peer transaction script function.
    let payload = encode_transfer_script_function(payee.clone(), amount);
    let function_call = ScriptFunctionCall::decode(&payload);
    match function_call {
        Some(ScriptFunctionCall::Transfer { amount: a, to: p }) => {
            assert_eq!(a, amount);
            assert_eq!(p, payee.clone());
        }
        _ => panic!("unexpected type of script function"),
    };

    let output = bcs::to_bytes(&payload).unwrap();
    for o in output {
        print!("{} ", o);
    }
    println!();
}

fn main() {
    demo_p2p_script_function();
}
