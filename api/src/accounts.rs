// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;

use diem_api_types::{Address, Error, LedgerInfo, MoveModuleBytecode, Response};
use diem_types::account_state::AccountState;

use anyhow::Result;
use serde_json::json;
use std::convert::{TryFrom, TryInto};
use warp::{Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_account_resources(context.clone()).or(get_account_modules(context))
}

// GET /accounts/<address>/resources
pub fn get_account_resources(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("accounts" / String / "resources")
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_get_account_resources)
}

// GET /accounts/<address>/modules
pub fn get_account_modules(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("accounts" / String / "modules")
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_get_account_modules)
}

async fn handle_get_account_resources(
    address: String,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Account::new(address, context)?.resources()?)
}

async fn handle_get_account_modules(
    address: String,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Account::new(address, context)?.modules()?)
}

struct Account {
    address: Address,
    ledger_info: LedgerInfo,
    context: Context,
}

impl Account {
    pub fn new(address: String, context: Context) -> Result<Self, Error> {
        Ok(Self {
            address: address.try_into().map_err(Error::bad_request)?,
            ledger_info: context.get_latest_ledger_info()?,
            context,
        })
    }

    pub fn resources(self) -> Result<impl Reply, Error> {
        let resources = self
            .context
            .move_converter()
            .try_into_resources(self.account_state()?.get_resources())?;
        Response::new(self.ledger_info, &resources)
    }

    pub fn modules(self) -> Result<impl Reply, Error> {
        let modules = self
            .account_state()?
            .get_modules()
            .map(MoveModuleBytecode::try_from)
            .collect::<Result<Vec<MoveModuleBytecode>>>()?;
        Response::new(self.ledger_info, &modules)
    }

    fn account_state(&self) -> Result<AccountState, Error> {
        let state = self
            .context
            .get_account_state(self.address.into(), self.ledger_info.version())?
            .ok_or_else(|| self.account_not_found())?;
        Ok(state)
    }

    fn account_not_found(&self) -> Error {
        Error::not_found(
            format!("could not find account by address: {}", self.address),
            json!({"ledger_version": self.ledger_info.ledger_version}),
        )
    }
}

#[cfg(any(test))]
mod tests {
    use crate::test_utils::{assert_json, find_value, new_test_context};
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
                "message": "could not find account by address: 0x0",
                "data": {
                    "ledger_version": info.ledger_version,
                },
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
                    "message": format!("invalid account address: {}", invalid_address),
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
            v["type"]["name"] == "Balance" && v["type"]["generic_type_params"][0]["name"] == "XDX"
        });
        assert_json(
            res,
            json!({
                "type": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "DiemAccount",
                    "name": "Balance",
                    "generic_type_params": [
                        {
                            "type": "struct",
                            "address": "0x1",
                            "module": "XDX",
                            "name": "XDX",
                            "generic_type_params": []
                        }
                    ]
                },
                "value": {
                    "coin": {
                        "value": "0"
                    }
                }
            }),
        );

        let res = find_value(&resp, |v| v["type"]["name"] == "EventHandleGenerator");
        assert_json(
            res,
            json!({
                "type": {
                    "type": "struct",
                    "address": "0x1",
                    "module": "Event",
                    "name": "EventHandleGenerator",
                    "generic_type_params": []
                },
                "value": {
                    "counter": "5",
                    "addr": "0xdd"
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
                            {
                                "type": "reference",
                                "mutable": false,
                                "to": {
                                    "type": "generic_type_param",
                                    "index": 0
                                }
                            }
                        ],
                        "return": [
                            {
                                "type": "vector",
                                "items": {
                                    "type": "u8"
                                }
                            }
                        ]
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
                            {"type": "signer"},
                            {"type": "signer"},
                            {"type": "u64"},
                            {
                                "type": "vector",
                                "items": {"type": "u8"}
                            }
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
                            {"type": "signer"},
                            {"type": "address"},
                            {"type": "u64"},
                            {
                                "type": "vector",
                                "items": {"type": "u8"}
                            },
                            {
                                "type": "vector",
                                "items": {"type": "u8"}
                            }
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
                    {
                        "address": "0x1",
                        "name": "DiemConsensusConfig"
                    },
                    {
                        "address": "0x1",
                        "name": "DiemSystem"
                    },
                    {
                        "address": "0x1",
                        "name": "DiemTransactionPublishingOption"
                    },
                    {
                        "address": "0x1",
                        "name": "DiemVMConfig"
                    },
                    {
                        "address": "0x1",
                        "name": "DiemVersion"
                    },
                    {
                        "address": "0x1",
                        "name": "RegisteredCurrencies"
                    }
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
                            {
                                "type": "generic_type_param",
                                "index": 0
                            }
                        ]
                    },
                    {
                        "name": "initialize",
                        "visibility": "public",
                        "generic_type_params": [],
                        "params": [
                            {
                                "type": "reference",
                                "mutable": false,
                                "to": {
                                    "type": "signer"
                                }
                            }
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
                            {
                                "type": "reference",
                                "mutable": false,
                                "to": {
                                    "type": "signer"
                                }
                            },
                            {
                                "type": "generic_type_param",
                                "index": 0
                            }
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
                            {
                                "type": "reference",
                                "mutable": false,
                                "to": {
                                    "type": "signer"
                                }
                            },
                            {
                                "type": "generic_type_param",
                                "index": 0
                            }
                        ],
                        "return": [
                            {
                                "type": "struct",
                                "address": "0x1",
                                "module": "DiemConfig",
                                "name": "ModifyConfigCapability",
                                "generic_type_params": [
                                    {
                                        "type": "generic_type_param",
                                        "index": 0
                                    }
                                ]
                            }
                        ]
                    },
                    {
                        "name": "reconfigure",
                        "visibility": "public",
                        "generic_type_params": [],
                        "params": [
                            {
                                "type": "reference",
                                "mutable": false,
                                "to": {
                                    "type": "signer"
                                }
                            }
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
                            {
                                "type": "reference",
                                "mutable": false,
                                "to": {
                                    "type": "signer"
                                }
                            },
                            {
                                "type": "generic_type_param",
                                "index": 0
                            }
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
                            {
                                "type": "reference",
                                "mutable": false,
                                "to": {
                                    "type": "struct",
                                    "address": "0x1",
                                    "module": "DiemConfig",
                                    "name": "ModifyConfigCapability",
                                    "generic_type_params": [
                                        {
                                            "type": "generic_type_param",
                                            "index": 0
                                        }
                                    ]
                                }
                            },
                            {
                                "type": "generic_type_param",
                                "index": 0
                            }
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
                                "type": {
                                    "type": "u64"
                                }
                            },
                            {
                                "name": "last_reconfiguration_time",
                                "type": {
                                    "type": "u64"
                                }
                            },
                            {
                                "name": "events",
                                "type": {
                                    "type": "struct",
                                    "address": "0x1",
                                    "module": "Event",
                                    "name": "EventHandle",
                                    "generic_type_params": [
                                        {
                                            "type": "struct",
                                            "address": "0x1",
                                            "module": "DiemConfig",
                                            "name": "NewEpochEvent",
                                            "generic_type_params": []
                                        }
                                    ]
                                }
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
                                "type": {
                                    "type": "generic_type_param",
                                    "index": 0
                                }
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
                                "type": {
                                    "type": "bool"
                                }
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
                                "type": {
                                    "type": "bool"
                                }
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
                                "type": {
                                    "type": "u64"
                                }
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
                        "type": {
                            "type": "struct",
                            "address": "0x1",
                            "module": "Diem",
                            "name": "Diem",
                            "generic_type_params": [
                                {
                                    "type": "generic_type_param",
                                    "index": 0
                                }
                            ]
                        }
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
                        "type": {
                            "type": "u64"
                        }
                    }
                ]
            }),
        );
    }

    fn account_resources(address: &str) -> String {
        format!("/accounts/{}/resources", address)
    }

    fn account_modules(address: &str) -> String {
        format!("/accounts/{}/modules", address)
    }
}
