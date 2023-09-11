---
title: "Account"
id: "cli-account"
---

## Account examples

### Fund an account with the faucet

You can fund an account with the faucet via the CLI by using either an account address or with `default` (which defaults to the account address created with `aptos init`).

For example, to fund the account `00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696` that was created above with the `aptos init` command:

```bash
$ aptos account fund-with-faucet --account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696
{
  "Result": "Added 10000 coins to account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696"
}
```

```bash
$ aptos account fund-with-faucet --account default
{
  "Result": "Added 10000 coins to account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696"
}
```

### View an account's balance and transfer events

You can view the balance and transfer events (deposits and withdrawals) either by explicitly specifying the account address, as below:

```bash
$ aptos account list --query balance --account 00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696
```

or by specifying the `default` as below:

```bash
$ aptos account list --query balance --account default
```

Both the above commands will generate the following information on your terminal:

```bash
{
  "Result": [
    {
      "coin": {
        "value": "110000"
      },
      "deposit_events": {
        "counter": "3",
        "guid": {
          "id": {
            "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
            "creation_num": "2"
          }
        }
      },
      "frozen": false,
      "withdraw_events": {
        "counter": "0",
        "guid": {
          "id": {
            "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
            "creation_num": "3"
          }
        }
      }
    }
  ]
}
```

### Listing resources in an account

You can list the resources in an account from the command line. For example, see below for how to list the resources in the account you just created above:

```bash
$ aptos account list --query resources --account default
```

or

```bash
$ aptos account list --query resources --account 0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696
```

Both the above commands will generate the following resource list information on your terminal:

```bash
{
  "Result": [
    {
      "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>": {
        "coin": {
          "value": "110000"
        },
        "deposit_events": {
          "counter": "3",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "2"
            }
          }
        },
        "frozen": false,
        "withdraw_events": {
          "counter": "0",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "3"
            }
          }
        }
      }
    },
    {
      "0x1::account::Account": {
        "authentication_key": "0x00f1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
        "coin_register_events": {
          "counter": "1",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "0"
            }
          }
        },
        "guid_creation_num": "4",
        "key_rotation_events": {
          "counter": "0",
          "guid": {
            "id": {
              "addr": "0xf1f20ddd0b0dd2291b6e42c97274668c479bca70f07c6b6a80b99720779696",
              "creation_num": "1"
            }
          }
        },
        "rotation_capability_offer": {
          "for": {
            "vec": []
          }
        },
        "sequence_number": "0",
        "signer_capability_offer": {
          "for": {
            "vec": []
          }
        }
      }
    }
  ]
}
```

### List the default profile

You can also list the default profile from configuration with no account specified.

:::tip
Account addresses may differ from example to example in this section.
:::

```bash
$ aptos account list
{
  "Result": [
    {
      "coin": {
        "value": "10000"
      },
      "deposit_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
            "creation_num": "1"
          }
        }
      },
      "withdraw_events": {
        "counter": "0",
        "guid": {
          "id": {
            "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
            "creation_num": "2"
          }
        }
      }
    },
    {
      "register_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
            "creation_num": "0"
          }
        }
      }
    },
    {
      "counter": "3"
    },
    {
      "authentication_key": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
      "self_address": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
      "sequence_number": "0"
    }
  ]
}
```

### Use the name of the profile

Additionally, any place that takes an account can use the name of a profile:

```bash
$ aptos account list --query resources --account superuser
{
  "Result": [
    {
      "coin": {
        "value": "10000"
      },
      "deposit_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
            "creation_num": "1"
          }
        }
      },
      "withdraw_events": {
        "counter": "0",
        "guid": {
          "id": {
            "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
            "creation_num": "2"
          }
        }
      }
    },
    {
      "register_events": {
        "counter": "1",
        "guid": {
          "id": {
            "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
            "creation_num": "0"
          }
        }
      }
    },
    {
      "counter": "3"
    },
    {
      "authentication_key": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
      "self_address": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
      "sequence_number": "0"
    }
  ]
}
```

### Listing modules in an account

You can pass different types of queries to view different items under an account. Currently, 'resources' and
'modules' are supported but more query types are coming. For example, to fetch modules:

```bash
$ aptos account list --query modules
{
  "Result": [
    {
      "bytecode": "0xa11ceb0b050000000b01000a020a12031c2504410405452d0772da0108cc0240068c030a0a9603150cab03650d90040400000101010201030104000506000006080004070700020e0401060100080001000009020300010f0404000410060100031107000002120709010602130a030106050806080105010802020c0a02000103040508020802070801010a0201060c010800010b0301090002070b030109000900074d657373616765056572726f72056576656e74067369676e657206737472696e67124d6573736167654368616e67654576656e740d4d657373616765486f6c64657206537472696e670b6765745f6d6573736167650b7365745f6d6573736167650c66726f6d5f6d6573736167650a746f5f6d657373616765076d657373616765156d6573736167655f6368616e67655f6576656e74730b4576656e7448616e646c65096e6f745f666f756e6404757466380a616464726573735f6f66106e65775f6576656e745f68616e646c650a656d69745f6576656e74b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb0000000000000000000000000000000000000000000000000000000000000001030800000000000000000002020a08020b08020102020c08020d0b030108000001000101030b0a002901030607001102270b002b0110001402010104010105240b0111030c040e0011040c020a02290120030b05120e000b040e00380012012d0105230b022a010c050a051000140c030a050f010b030a04120038010b040b050f0015020100010100",
      "abi": {
        "address": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
        "name": "Message",
        "friends": [],
        "exposed_functions": [
          {
            "name": "get_message",
            "visibility": "public",
            "is_entry": false,
            "generic_type_params": [],
            "params": [
              "address"
            ],
            "return": [
              "0x1::string::String"
            ]
          },
          {
            "name": "set_message",
            "visibility": "public",
            "is_entry": true,
            "generic_type_params": [],
            "params": [
              "signer",
              "vector<u8>"
            ],
            "return": []
          }
        ],
        "structs": [
          {
            "name": "MessageChangeEvent",
            "is_native": false,
            "abilities": [
              "drop",
              "store"
            ],
            "generic_type_params": [],
            "fields": [
              {
                "name": "from_message",
                "type": "0x1::string::String"
              },
              {
                "name": "to_message",
                "type": "0x1::string::String"
              }
            ]
          },
          {
            "name": "MessageHolder",
            "is_native": false,
            "abilities": [
              "key"
            ],
            "generic_type_params": [],
            "fields": [
              {
                "name": "message",
                "type": "0x1::string::String"
              },
              {
                "name": "message_change_events",
                "type": "0x1::event::EventHandle<0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb::Message::MessageChangeEvent>"
              }
            ]
          }
        ]
      }
    }
  ]
}
```

### Transferring coins

The Aptos CLI is a simple wallet as well, and can transfer coins between accounts.

```bash
$ aptos account transfer --account superuser --amount 100
{
  "Result": {
    "gas_used": 73,
    "balance_changes": {
      "742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc": {
        "coin": {
          "value": "10100"
        },
        "deposit_events": {
          "counter": "2",
          "guid": {
            "id": {
              "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
              "creation_num": "1"
            }
          }
        },
        "withdraw_events": {
          "counter": "0",
          "guid": {
            "id": {
              "addr": "0x742854f7dca56ea6309b51e8cebb830b12623f9c9d76c72c3242e4cad353dedc",
              "creation_num": "2"
            }
          }
        }
      },
      "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb": {
        "coin": {
          "value": "9827"
        },
        "deposit_events": {
          "counter": "1",
          "guid": {
            "id": {
              "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
              "creation_num": "1"
            }
          }
        },
        "withdraw_events": {
          "counter": "1",
          "guid": {
            "id": {
              "addr": "0xb9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
              "creation_num": "2"
            }
          }
        }
      }
    },
    "sender": "b9bd2cfa58ca29bce1d7add25fce5c62220604cd0236fe3f90d9de91ed9fb8cb",
    "success": true,
    "version": 1139,
    "vm_status": "Executed successfully"
  }
}
```