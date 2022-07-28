---
id: "Types.ScriptFunctionPayload"
title: "Interface: ScriptFunctionPayload"
sidebar_label: "ScriptFunctionPayload"
custom_edit_url: null
---

[Types](../namespaces/Types.md).ScriptFunctionPayload

**`example`** {"type":"script_function_payload","function":"0x1::PaymentScripts::peer_to_peer_with_metadata","type_arguments":["0x1::XDX::XDX"],"arguments":["0x1668f6be25668c1a17cd8caf6b8d2f25","2021000000","0x","0x"]}

## Properties

### arguments

• **arguments**: `any`[]

The script function arguments.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:580](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L580)

___

### function

• **function**: `string`

Script function id is string representation of a script function defined on-chain.

Format: `{address}::{module name}::{function name}`
Both `module name` and `function name` are case-sensitive.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:574](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L574)

___

### type

• **type**: `string`

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:566](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L566)

___

### type\_arguments

• **type\_arguments**: `string`[]

Generic type arguments required by the script function.

#### Defined in

[ecosystem/typescript/sdk/src/api/data-contracts.ts:577](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/api/data-contracts.ts#L577)
