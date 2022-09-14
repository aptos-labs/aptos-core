// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use aptos_api_types::Address;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::types::LocalAccount;
use serde_json::json;

use std::convert::TryInto;

#[tokio::test]
#[ignore] // TODO(issue 81): re-enable this test when having correct script code
async fn test_renders_move_acsii_string_into_utf8_string() {
    let mut context = new_test_context(current_function_name!());
    let mut account = init_test_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn]).await;

    // module 0x87342d91af60c3a883a2812c9294c2f8::Message {
    //     use Std::ascii;
    //     struct MessageHolder has key {
    //         message: string::String,
    //     }
    //     public(script) fun set_message(account: signer, msg: vector<u8>) {
    //         let message = string::utf8(msg);
    //         move_to(&account, MessageHolder {
    //             message,
    //         });
    //     }
    // }
    let module_code = "0xa11ceb0b0400000008010004020408030c0a05160b07213e085f200a7f060c85011500000101000208000105070000030001000106030200020c0a0200010801010a02074d6573736167650541534349490d4d657373616765486f6c6465720b7365745f6d657373616765076d65737361676506537472696e6706737472696e6787342d91af60c3a883a2812c9294c2f8000000000000000000000000000000010002010408010002000002080b0111010c020e000b0212002d000200";
    context
        .api_publish_module(&mut account, module_code.parse().unwrap())
        .await;

    context
        .api_execute_entry_function(
            &mut account,
            "Message",
            "set_message",
            json!([]),
            json!([hex::encode(b"hello world")]),
        )
        .await;

    let message = context
        .api_get_account_resource(
            &account,
            &account.address().to_hex_literal(),
            "Message",
            "MessageHolder",
        )
        .await;
    assert_eq!("hello world", message["data"]["message"]);
}

fn init_test_account() -> LocalAccount {
    let key_bytes =
        hex::decode("a38ba78b1a0fbfc55e2c5dfdedf48d1172283d0f7c59fd64c02d811130a2f4b2").unwrap();
    let private_key: Ed25519PrivateKey = (&key_bytes[..]).try_into().unwrap();
    let address: Address = "0x87342d91af60c3a883a2812c9294c2f8".parse().unwrap();
    LocalAccount::new(address.into(), private_key, 0)
}
