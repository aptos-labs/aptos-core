---
id: "RequestError"
title: "Class: RequestError"
sidebar_label: "RequestError"
sidebar_position: 0
custom_edit_url: null
---

## Hierarchy

- `Error`

  ↳ **`RequestError`**

## Constructors

### constructor

• **new RequestError**(`message?`, `response?`, `requestBody?`)

#### Parameters

| Name | Type |
| :------ | :------ |
| `message?` | `string` |
| `response?` | `AxiosResponse`<`any`, [`AptosError`](../interfaces/Types.AptosError.md)\> |
| `requestBody?` | `string` |

#### Overrides

Error.constructor

#### Defined in

[ecosystem/typescript/sdk/src/aptos_client.ts:19](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_client.ts#L19)

## Properties

### message

• **message**: `string`

#### Inherited from

Error.message

#### Defined in

developer-docs-site/node_modules/typescript/lib/lib.es5.d.ts:1023

___

### name

• **name**: `string`

#### Inherited from

Error.name

#### Defined in

developer-docs-site/node_modules/typescript/lib/lib.es5.d.ts:1022

___

### requestBody

• `Optional` **requestBody**: `string`

#### Defined in

[ecosystem/typescript/sdk/src/aptos_client.ts:17](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_client.ts#L17)

___

### response

• `Optional` **response**: `AxiosResponse`<`any`, [`AptosError`](../interfaces/Types.AptosError.md)\>

#### Defined in

[ecosystem/typescript/sdk/src/aptos_client.ts:15](https://github.com/aptos-labs/aptos-core/blob/fb73eb358/ecosystem/typescript/sdk/src/aptos_client.ts#L15)

___

### stack

• `Optional` **stack**: `string`

#### Inherited from

Error.stack

#### Defined in

developer-docs-site/node_modules/typescript/lib/lib.es5.d.ts:1024

___

### prepareStackTrace

▪ `Static` `Optional` **prepareStackTrace**: (`err`: `Error`, `stackTraces`: `CallSite`[]) => `any`

#### Type declaration

▸ (`err`, `stackTraces`): `any`

Optional override for formatting stack traces

**`see`** https://v8.dev/docs/stack-trace-api#customizing-stack-traces

##### Parameters

| Name | Type |
| :------ | :------ |
| `err` | `Error` |
| `stackTraces` | `CallSite`[] |

##### Returns

`any`

#### Inherited from

Error.prepareStackTrace

#### Defined in

ecosystem/typescript/sdk/node_modules/@types/node/globals.d.ts:11

___

### stackTraceLimit

▪ `Static` **stackTraceLimit**: `number`

#### Inherited from

Error.stackTraceLimit

#### Defined in

ecosystem/typescript/sdk/node_modules/@types/node/globals.d.ts:13

## Methods

### captureStackTrace

▸ `Static` **captureStackTrace**(`targetObject`, `constructorOpt?`): `void`

Create .stack property on a target object

#### Parameters

| Name | Type |
| :------ | :------ |
| `targetObject` | `object` |
| `constructorOpt?` | `Function` |

#### Returns

`void`

#### Inherited from

Error.captureStackTrace

#### Defined in

ecosystem/typescript/sdk/node_modules/@types/node/globals.d.ts:4
