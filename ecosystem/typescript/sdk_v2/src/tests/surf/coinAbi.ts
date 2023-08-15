export type CoinAbi = {
    "address": "0x1",
    "name": "coin",
    "friends": [
        "0x1::aptos_coin",
        "0x1::genesis",
        "0x1::transaction_fee"
    ],
    "exposed_functions": [
        {
            "name": "allow_supply_upgrades",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [],
            "params": [
                "&signer",
                "bool"
            ],
            "return": []
        },
        {
            "name": "balance",
            "visibility": "public",
            "is_entry": false,
            "is_view": true,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "address"
            ],
            "return": [
                "u64"
            ]
        },
        {
            "name": "burn",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "0x1::coin::Coin<T0>",
                "&0x1::coin::BurnCapability<T0>"
            ],
            "return": []
        },
        {
            "name": "burn_from",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "address",
                "u64",
                "&0x1::coin::BurnCapability<T0>"
            ],
            "return": []
        },
        {
            "name": "collect_into_aggregatable_coin",
            "visibility": "friend",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "address",
                "u64",
                "&mut 0x1::coin::AggregatableCoin<T0>"
            ],
            "return": []
        },
        {
            "name": "decimals",
            "visibility": "public",
            "is_entry": false,
            "is_view": true,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [],
            "return": [
                "u8"
            ]
        },
        {
            "name": "deposit",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "address",
                "0x1::coin::Coin<T0>"
            ],
            "return": []
        },
        {
            "name": "destroy_burn_cap",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "0x1::coin::BurnCapability<T0>"
            ],
            "return": []
        },
        {
            "name": "destroy_freeze_cap",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "0x1::coin::FreezeCapability<T0>"
            ],
            "return": []
        },
        {
            "name": "destroy_mint_cap",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "0x1::coin::MintCapability<T0>"
            ],
            "return": []
        },
        {
            "name": "destroy_zero",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "0x1::coin::Coin<T0>"
            ],
            "return": []
        },
        {
            "name": "drain_aggregatable_coin",
            "visibility": "friend",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&mut 0x1::coin::AggregatableCoin<T0>"
            ],
            "return": [
                "0x1::coin::Coin<T0>"
            ]
        },
        {
            "name": "extract",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&mut 0x1::coin::Coin<T0>",
                "u64"
            ],
            "return": [
                "0x1::coin::Coin<T0>"
            ]
        },
        {
            "name": "extract_all",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&mut 0x1::coin::Coin<T0>"
            ],
            "return": [
                "0x1::coin::Coin<T0>"
            ]
        },
        {
            "name": "freeze_coin_store",
            "visibility": "public",
            "is_entry": true,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "address",
                "&0x1::coin::FreezeCapability<T0>"
            ],
            "return": []
        },
        {
            "name": "initialize",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&signer",
                "0x1::string::String",
                "0x1::string::String",
                "u8",
                "bool"
            ],
            "return": [
                "0x1::coin::BurnCapability<T0>",
                "0x1::coin::FreezeCapability<T0>",
                "0x1::coin::MintCapability<T0>"
            ]
        },
        {
            "name": "initialize_aggregatable_coin",
            "visibility": "friend",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&signer"
            ],
            "return": [
                "0x1::coin::AggregatableCoin<T0>"
            ]
        },
        {
            "name": "initialize_supply_config",
            "visibility": "friend",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [],
            "params": [
                "&signer"
            ],
            "return": []
        },
        {
            "name": "initialize_with_parallelizable_supply",
            "visibility": "friend",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&signer",
                "0x1::string::String",
                "0x1::string::String",
                "u8",
                "bool"
            ],
            "return": [
                "0x1::coin::BurnCapability<T0>",
                "0x1::coin::FreezeCapability<T0>",
                "0x1::coin::MintCapability<T0>"
            ]
        },
        {
            "name": "is_account_registered",
            "visibility": "public",
            "is_entry": false,
            "is_view": true,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "address"
            ],
            "return": [
                "bool"
            ]
        },
        {
            "name": "is_aggregatable_coin_zero",
            "visibility": "friend",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&0x1::coin::AggregatableCoin<T0>"
            ],
            "return": [
                "bool"
            ]
        },
        {
            "name": "is_coin_initialized",
            "visibility": "public",
            "is_entry": false,
            "is_view": true,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [],
            "return": [
                "bool"
            ]
        },
        {
            "name": "merge",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&mut 0x1::coin::Coin<T0>",
                "0x1::coin::Coin<T0>"
            ],
            "return": []
        },
        {
            "name": "merge_aggregatable_coin",
            "visibility": "friend",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&mut 0x1::coin::AggregatableCoin<T0>",
                "0x1::coin::Coin<T0>"
            ],
            "return": []
        },
        {
            "name": "mint",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "u64",
                "&0x1::coin::MintCapability<T0>"
            ],
            "return": [
                "0x1::coin::Coin<T0>"
            ]
        },
        {
            "name": "name",
            "visibility": "public",
            "is_entry": false,
            "is_view": true,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [],
            "return": [
                "0x1::string::String"
            ]
        },
        {
            "name": "register",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&signer"
            ],
            "return": []
        },
        {
            "name": "supply",
            "visibility": "public",
            "is_entry": false,
            "is_view": true,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [],
            "return": [
                "0x1::option::Option<u128>"
            ]
        },
        {
            "name": "symbol",
            "visibility": "public",
            "is_entry": false,
            "is_view": true,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [],
            "return": [
                "0x1::string::String"
            ]
        },
        {
            "name": "transfer",
            "visibility": "public",
            "is_entry": true,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&signer",
                "address",
                "u64"
            ],
            "return": []
        },
        {
            "name": "unfreeze_coin_store",
            "visibility": "public",
            "is_entry": true,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "address",
                "&0x1::coin::FreezeCapability<T0>"
            ],
            "return": []
        },
        {
            "name": "upgrade_supply",
            "visibility": "public",
            "is_entry": true,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&signer"
            ],
            "return": []
        },
        {
            "name": "value",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&0x1::coin::Coin<T0>"
            ],
            "return": [
                "u64"
            ]
        },
        {
            "name": "withdraw",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [
                "&signer",
                "u64"
            ],
            "return": [
                "0x1::coin::Coin<T0>"
            ]
        },
        {
            "name": "zero",
            "visibility": "public",
            "is_entry": false,
            "is_view": false,
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "params": [],
            "return": [
                "0x1::coin::Coin<T0>"
            ]
        }
    ],
    "structs": [
        {
            "name": "AggregatableCoin",
            "is_native": false,
            "abilities": [
                "store"
            ],
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "fields": [
                {
                    "name": "value",
                    "type": "0x1::aggregator::Aggregator"
                }
            ]
        },
        {
            "name": "BurnCapability",
            "is_native": false,
            "abilities": [
                "copy",
                "store"
            ],
            "generic_type_params": [
                {
                    "constraints": []
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
            "name": "Coin",
            "is_native": false,
            "abilities": [
                "store"
            ],
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "fields": [
                {
                    "name": "value",
                    "type": "u64"
                }
            ]
        },
        {
            "name": "CoinInfo",
            "is_native": false,
            "abilities": [
                "key"
            ],
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "fields": [
                {
                    "name": "name",
                    "type": "0x1::string::String"
                },
                {
                    "name": "symbol",
                    "type": "0x1::string::String"
                },
                {
                    "name": "decimals",
                    "type": "u8"
                },
                {
                    "name": "supply",
                    "type": "0x1::option::Option<0x1::optional_aggregator::OptionalAggregator>"
                }
            ]
        },
        {
            "name": "CoinStore",
            "is_native": false,
            "abilities": [
                "key"
            ],
            "generic_type_params": [
                {
                    "constraints": []
                }
            ],
            "fields": [
                {
                    "name": "coin",
                    "type": "0x1::coin::Coin<T0>"
                },
                {
                    "name": "frozen",
                    "type": "bool"
                },
                {
                    "name": "deposit_events",
                    "type": "0x1::event::EventHandle<0x1::coin::DepositEvent>"
                },
                {
                    "name": "withdraw_events",
                    "type": "0x1::event::EventHandle<0x1::coin::WithdrawEvent>"
                }
            ]
        },
        {
            "name": "DepositEvent",
            "is_native": false,
            "abilities": [
                "drop",
                "store"
            ],
            "generic_type_params": [],
            "fields": [
                {
                    "name": "amount",
                    "type": "u64"
                }
            ]
        },
        {
            "name": "FreezeCapability",
            "is_native": false,
            "abilities": [
                "copy",
                "store"
            ],
            "generic_type_params": [
                {
                    "constraints": []
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
            "name": "MintCapability",
            "is_native": false,
            "abilities": [
                "copy",
                "store"
            ],
            "generic_type_params": [
                {
                    "constraints": []
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
            "name": "SupplyConfig",
            "is_native": false,
            "abilities": [
                "key"
            ],
            "generic_type_params": [],
            "fields": [
                {
                    "name": "allow_upgrades",
                    "type": "bool"
                }
            ]
        },
        {
            "name": "WithdrawEvent",
            "is_native": false,
            "abilities": [
                "drop",
                "store"
            ],
            "generic_type_params": [],
            "fields": [
                {
                    "name": "amount",
                    "type": "u64"
                }
            ]
        }
    ]
};
