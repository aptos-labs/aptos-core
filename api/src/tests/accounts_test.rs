// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{assert_json, find_value, new_test_context};
use serde_json::json;

#[tokio::test]
async fn test_get_account_resources_returns_empty_array_for_account_has_no_resources() {
    let context = new_test_context();
    let address = "0x1";

    let resp = context.get(&account_resources(address)).await;
    assert_eq!(json!([]), resp);
}

#[tokio::test]
async fn test_get_account_resources_by_address_0x0() {
    let context = new_test_context();
    let address = "0x0";

    let info = context.get_latest_ledger_info();
    let resp = context
        .expect_status_code(404)
        .get(&account_resources(address))
        .await;
    assert_eq!(
        json!({
            "code": 404,
            "message": "account not found by address(0x0) and ledger version(0)",
            "diem_ledger_version": info.ledger_version,
        }),
        resp
    );
}

#[tokio::test]
async fn test_get_account_resources_by_invalid_address_missing_0x_prefix() {
    let context = new_test_context();
    let invalid_addresses = vec!["1", "0xzz", "01"];
    for invalid_address in &invalid_addresses {
        let resp = context
            .expect_status_code(400)
            .get(&account_resources(invalid_address))
            .await;
        assert_eq!(
            json!({
                "code": 400,
                "message": format!("invalid parameter account address: {}", invalid_address),
            }),
            resp
        );
    }
}

#[tokio::test]
async fn test_get_account_resources_by_valid_account_address() {
    let context = new_test_context();
    let addresses = vec![
        "0xdd",
        "000000000000000000000000000000dd",
        "0x000000000000000000000000000000dd",
    ];
    for address in &addresses {
        context.get(&account_resources(address)).await;
    }
}

#[tokio::test]
async fn test_account_resources_response() {
    let context = new_test_context();
    let address = "0xdd";

    let resp = context.get(&account_resources(address)).await;

    let res = find_value(&resp, |v| {
        v["type"] == "0x1::DiemAccount::Balance<0x1::XDX::XDX>"
    });
    assert_json(
        res,
        json!({
            "type": "0x1::DiemAccount::Balance<0x1::XDX::XDX>",
            "value": {
                "coin": {
                    "value": "0"
                }
            }
        }),
    );
}

#[tokio::test]
async fn test_account_modules() {
    let context = new_test_context();
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;

    let res = find_value(&resp, |v| v["abi"]["name"] == "BCS");
    assert!(res["bytecode"].as_str().unwrap().starts_with("0x"));
    assert_json(
        res["abi"].clone(),
        json!({
            "address": "0x1",
            "name": "BCS",
            "friends": [],
            "exposed_functions": [
                {
                    "name": "to_bytes",
                    "visibility": "public",
                    "generic_type_params": [
                        {
                            "constraints": []
                        }
                    ],
                    "params": [
                        "&T0"
                    ],
                    "return": ["vector<u8>"]
                }
            ],
            "structs": []
        }),
    );
}

#[tokio::test]
async fn test_get_module_with_script_functions() {
    let context = new_test_context();
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    let res = find_value(&resp, |v| v["abi"]["name"] == "PaymentScripts");
    assert_json(
        res["abi"].clone(),
        json!({
            "address": "0x1",
            "name": "PaymentScripts",
            "friends": [],
            "exposed_functions": [
                {
                    "name": "peer_to_peer_by_signers",
                    "visibility": "script",
                    "generic_type_params": [
                        {
                            "constraints": []
                        }
                    ],
                    "params": [
                        "signer",
                        "signer",
                        "u64",
                        "vector<u8>"
                    ],
                    "return": []
                },
                {
                    "name": "peer_to_peer_with_metadata",
                    "visibility": "script",
                    "generic_type_params": [
                        {
                            "constraints": []
                        }
                    ],
                    "params": [
                        "signer",
                        "address",
                        "u64",
                        "vector<u8>",
                        "vector<u8>"
                    ],
                    "return": []
                }
            ],
            "structs": []
        }),
    );
}

#[tokio::test]
async fn test_get_module_diem_config() {
    let context = new_test_context();
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    let res = find_value(&resp, |v| v["abi"]["name"] == "DiemConfig");
    assert_json(
        res["abi"].clone(),
        json!({
            "address": "0x1",
            "name": "DiemConfig",
            "friends": [
                "0x1::DiemConsensusConfig",
                "0x1::DiemSystem",
                "0x1::DiemTransactionPublishingOption",
                "0x1::DiemVMConfig",
                "0x1::DiemVersion",
                "0x1::ParallelExecutionConfig",
                "0x1::RegisteredCurrencies"
            ],
            "exposed_functions": [
                {
                    "name": "get",
                    "visibility": "public",
                    "generic_type_params": [
                        {
                            "constraints": [
                                "copy",
                                "drop",
                                "store"
                            ]
                        }
                    ],
                    "params": [],
                    "return": [
                        "T0"
                    ]
                },
                {
                    "name": "initialize",
                    "visibility": "public",
                    "generic_type_params": [],
                    "params": [
                        "&signer"
                    ],
                    "return": []
                },
                {
                    "name": "publish_new_config",
                    "visibility": "friend",
                    "generic_type_params": [
                        {
                            "constraints": [
                                "copy",
                                "drop",
                                "store"
                            ]
                        }
                    ],
                    "params": [
                        "&signer",
                        "T0"
                    ],
                    "return": []
                },
                {
                    "name": "publish_new_config_and_get_capability",
                    "visibility": "friend",
                    "generic_type_params": [
                        {
                            "constraints": [
                                "copy",
                                "drop",
                                "store"
                            ]
                        }
                    ],
                    "params": [
                        "&signer",
                        "T0"
                    ],
                    "return": [
                        "0x1::DiemConfig::ModifyConfigCapability<T0>"
                    ]
                },
                {
                    "name": "reconfigure",
                    "visibility": "public",
                    "generic_type_params": [],
                    "params": [
                        "&signer"
                    ],
                    "return": []
                },
                {
                    "name": "set",
                    "visibility": "friend",
                    "generic_type_params": [
                        {
                            "constraints": [
                                "copy",
                                "drop",
                                "store"
                            ]
                        }
                    ],
                    "params": [
                        "&signer",
                        "T0"
                    ],
                    "return": []
                },
                {
                    "name": "set_with_capability_and_reconfigure",
                    "visibility": "friend",
                    "generic_type_params": [
                        {
                            "constraints": [
                                "copy",
                                "drop",
                                "store"
                            ]
                        }
                    ],
                    "params": [
                        "&0x1::DiemConfig::ModifyConfigCapability<T0>",
                        "T0"
                    ],
                    "return": []
                }
            ],
            "structs": [
                {
                    "name": "Configuration",
                    "is_native": false,
                    "abilities": [
                        "key"
                    ],
                    "generic_type_params": [],
                    "fields": [
                        {
                            "name": "epoch",
                            "type": "u64"
                        },
                        {
                            "name": "last_reconfiguration_time",
                            "type": "u64"
                        },
                        {
                            "name": "events",
                            "type": "0x1::Event::EventHandle<0x1::DiemConfig::NewEpochEvent>"
                        }
                    ]
                },
                {
                    "name": "DiemConfig",
                    "is_native": false,
                    "abilities": [
                        "store",
                        "key"
                    ],
                    "generic_type_params": [
                        {
                            "constraints": [
                                "copy",
                                "drop",
                                "store"
                            ],
                            "is_phantom": false
                        }
                    ],
                    "fields": [
                        {
                            "name": "payload",
                            "type": "T0"
                        }
                    ]
                },
                {
                    "name": "DisableReconfiguration",
                    "is_native": false,
                    "abilities": [
                        "key"
                    ],
                    "generic_type_params": [],
                    "fields": [
                        {
                            "name": "dummy_field",
                            "type": "bool"
                        }
                    ]
                },
                {
                    "name": "ModifyConfigCapability",
                    "is_native": false,
                    "abilities": [
                        "store",
                        "key"
                    ],
                    "generic_type_params": [
                        {
                            "constraints": [],
                            "is_phantom": true
                        }
                    ],
                    "fields": [
                        {
                            "name": "dummy_field",
                            "type": "bool"
                        }
                    ]
                },
                {
                    "name": "NewEpochEvent",
                    "is_native": false,
                    "abilities": [
                        "drop",
                        "store"
                    ],
                    "generic_type_params": [],
                    "fields": [
                        {
                            "name": "epoch",
                            "type": "u64"
                        }
                    ]
                }
            ]
        }),
    );
}

#[tokio::test]
async fn test_account_modules_structs() {
    let context = new_test_context();
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;

    let diem_account_module = find_value(&resp, |v| v["abi"]["name"] == "DiemAccount");
    let balance_struct = find_value(&diem_account_module["abi"]["structs"], |v| {
        v["name"] == "Balance"
    });
    assert_json(
        balance_struct,
        json!({
            "name": "Balance",
            "is_native": false,
            "abilities": [
                "key"
            ],
            "generic_type_params": [
                {
                    "constraints": [],
                    "is_phantom": true
                }
            ],
            "fields": [
                {
                    "name": "coin",
                    "type": "0x1::Diem::Diem<T0>"
                }
            ]
        }),
    );

    let diem_module = find_value(&resp, |f| f["abi"]["name"] == "Diem");
    let diem_struct = find_value(&diem_module["abi"]["structs"], |f| f["name"] == "Diem");
    assert_json(
        diem_struct,
        json!({
            "name": "Diem",
            "is_native": false,
            "abilities": [
                "store"
            ],
            "generic_type_params": [
                {
                    "constraints": [],
                    "is_phantom": true
                }
            ],
            "fields": [
                {
                    "name": "value",
                    "type": "u64"
                }
            ]
        }),
    );
}

#[tokio::test]
async fn test_get_account_resources_by_ledger_version() {
    let mut context = new_test_context();
    let account = context.gen_account();
    let txn = context.create_parent_vasp(&account);
    context.commit_block(&vec![txn.clone()]);

    let ledger_version_1_resources = context
        .get(&account_resources(
            &context.tc_account().address().to_hex_literal(),
        ))
        .await;
    let tc_account = find_value(&ledger_version_1_resources, |f| {
        f["type"] == "0x1::DiemAccount::DiemAccount"
    });
    assert_eq!(tc_account["value"]["sequence_number"], "1");

    let ledger_version_0_resources = context
        .get(&account_resources_with_ledger_version(
            &context.tc_account().address().to_hex_literal(),
            0,
        ))
        .await;
    let tc_account = find_value(&ledger_version_0_resources, |f| {
        f["type"] == "0x1::DiemAccount::DiemAccount"
    });
    assert_eq!(tc_account["value"]["sequence_number"], "0");
}

#[tokio::test]
async fn test_get_account_resources_by_ledger_version_is_too_large() {
    let context = new_test_context();
    let resp = context
        .expect_status_code(404)
        .get(&account_resources_with_ledger_version(
            &context.tc_account().address().to_hex_literal(),
            1000000000000000000,
        ))
        .await;
    assert_json(
        resp,
        json!({
            "code": 404,
            "message": "ledger not found by version(1000000000000000000)",
            "diem_ledger_version": "0"
        }),
    );
}

#[tokio::test]
async fn test_get_account_resources_by_invalid_ledger_version() {
    let context = new_test_context();
    let resp = context
        .expect_status_code(400)
        .get(&account_resources_with_ledger_version(
            &context.tc_account().address().to_hex_literal(),
            -1,
        ))
        .await;
    assert_json(
        resp,
        json!({
            "code": 400,
            "message": "invalid parameter ledger version: -1"
        }),
    );
}

#[tokio::test]
async fn test_get_account_modules_by_ledger_version() {
    let context = new_test_context();
    let code = "a11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200";
    let mut tc_account = context.tc_account();
    let txn = tc_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .module(hex::decode(code).unwrap()),
    );
    context.commit_block(&vec![txn.clone()]);

    let modules = context
        .get(&account_modules(
            &context.tc_account().address().to_hex_literal(),
        ))
        .await;
    assert_ne!(modules, json!([]));

    let modules = context
        .get(&account_modules_with_ledger_version(
            &context.tc_account().address().to_hex_literal(),
            0,
        ))
        .await;
    assert_eq!(modules, json!([]));
}

fn account_resources(address: &str) -> String {
    format!("/accounts/{}/resources", address)
}

fn account_resources_with_ledger_version(address: &str, ledger_version: i128) -> String {
    format!("/ledger/{}{}", ledger_version, account_resources(address))
}

fn account_modules(address: &str) -> String {
    format!("/accounts/{}/modules", address)
}

fn account_modules_with_ledger_version(address: &str, ledger_version: i128) -> String {
    format!("/ledger/{}{}", ledger_version, account_modules(address))
}
