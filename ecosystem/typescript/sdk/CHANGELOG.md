# Aptos TS SDK Changelog

All notable changes to the Aptos Node SDK will be captured in this file. This changelog is written by hand for now. It adheres to the format set out by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

**Note:** The Aptos TS SDK does not follow semantic version while we are in active development. Instead, breaking changes will be announced with each devnet cut. Once we launch our mainnet, the SDK will follow semantic versioning closely.

## Unreleased

N/A

## 1.4.0 (2022-11-30)

- Add missing fields to TokenData class
- Add PropertyMap and PropertyValue type to match on-chain data
- Support token property map deseralizer to read the property map in the original data format.
- Allow `checkBalance` in `CoinClient` to take in a `MaybeHexString` as well as `AptosAccount`, since users might want to check the balance of accounts they don't own (which is generally how you use `AptosAccount`).
- Similar to `checkBalance`, allow `transfer` in `CoinClient` to take in a `MaybeHexString` for the `receiver` argument.
- Add a new `createReceiverIfMissing` argument to `transfer` in `CoinClient`. If set, the `0x1::aptos_account::transfer` function will be called instead of `0x1::coin::transfer`, which will create the account on chain if it doesn't exist instead of failing.


## 1.3.17 (2022-11-08)

- Support computing resource account address based off a source address and a seed
- Exported ABI types
- `getAccountModules` and `getAccountResources` now use pagination under the hood. This addresses the issue raised here: https://github.com/aptos-labs/aptos-core/issues/5298. The changes are non-breaking, if you use these functions with an older node that hasn't updated to include the relevant support in its API service, it will still work as it did before.
- To support the above, the generated client has been updated to attach the headers to the response object, as per the changes here: https://github.com/aptos-labs/openapi-typescript-codegen/compare/v0.23.0...aptos-labs:openapi-typescript-codegen:0.24.0?expand=1. Consider this an implementation detail, not a supported part of the SDK interface.
- Add functions to token client support
  - direct transfer with opt-in
  - burn token by owner
  - burn token by creator
  - mutate token properties
- Add property map serializer to serialize input to BCS encode

## 1.3.16 (2022-10-12)

- Add `estimatePrioritizedGasUnitPrice` to the simulation interface. If set to true, the estimated gas unit price is higher than the original estimate. Therefore, transactions have a higher chance to be executed during congestion period.
- `esitmateGasPrice` now returns `deprioritized_gas_estimate` and `prioritized_gas_estimate` along with `gas_estimate`. `deprioritized_gas_estimate` is a conservative price estimate. Users might end up paying less gas eventually, but the transaction execution is deprioritized by the block chain. On the other hand, `prioritized_gas_estimate` is a higher price esitmate. Transactions need to be executed sooner could use `prioritized_gas_estimate`.

## 1.3.15 (2022-09-30)

- **[Breaking Changes]** Following the deprecation notice in the release notes of 1.3.13, the following breaking changes have landed in this release. Please see the notes from last release for information on the new endpoints you must migrate to:
  - The `getEventsByEventKey` function has been removed.
  - The `key` field in the `Event` struct has been removed.
- Turn on `strict` in tsconfig

## 1.3.14 (2022-09-20)

- Enable SDK axios client to carry cookies for both the browser and node environments.
- Added new functions `getBlockByHeight` and `getBlockByVersion`.

## 1.3.13 (2022-09-15)

- Increase the default wait time for `waitForTransactionWithResult` to 20s.
- A new function called `getEventsByCreationNumber` has been added, corresponding to the new endpoint on the API. For more information on this change, see the [API changelog](https://github.com/aptos-labs/aptos-core/blob/main/api/doc/CHANGELOG.md) for API version 1.1.0.
- **[Deprecated]** The `getEventsByEventKey` function is now deprecated. In the next release it will be removed entirely. You must migrate to the new function, `getEventsByCreationNumber`, by then.
- Included in the `Event` struct (which is what the events endpoints return) is a new field called `guid`. This is a more easily interpretable representation of an event identifier than the `key` field. See the [API changelog](https://github.com/aptos-labs/aptos-core/blob/main/api/doc/CHANGELOG.md) for an example of the new field.
- **[Deprecated]** The `key` field in the `Event` struct is now deprecated. In the next release it will be removed entirely. You must migrate to using the `guid` field by then.
- Removed NPM dependencies ed25519-hd-key and typescript-memoize.
- Added IIFE bundle that can be served from CDN. No NPM is required to use the SDK in browser environment.

## 1.3.12 (2022-09-08)

- Feature to rotate auth key for single signature account

## 1.3.11 (2022-08-31)

- Upgraded typescript version from 4.7.4 to 4.8.2, as well as linter package versions.
- **[Breaking Change]** ModuleBundle transaction support is removed. Instead, SDK users should use `AptosClient.publishPackage` to publish Move packages.
- Expose detailed API errors.
- Accept stringified values as transaction payload parameters.

## 1.3.10 (2022-08-26)

- Fix the bug in `waitForTransactionWithResult`. When API returns `404`, the function should continue waiting rather than returning early. The reason is that the txn might not be committed promptly. `waitForTransactionWithResult` should either timeout or get an error in such case.

## 1.3.9 (2022-08-25)

- **[Breaking Change]** Reimplemented the JSON transaction submission interfaces with BCS. This is a breaking change. `createSigningMessage` is removed. Before the changes, the transaction payloads take string aruguments. But now, Typescript payload arguments have to match the smart contract arugment types. e.g. `number` matches `u8`, `number | bigint` matches `u64` and `u128`, etc.
- **[Breaking Change]** `getTokenBalance` and `getTokenBalanceForAccount` have been renamed to `getToken` and `getTokenForAccount`, since they were never getting just the balance, but the full token.
- Added `CoinClient` to help working with coins. This contains common operations such as `transfer`, `checkBalance`, etc.
- Added `generateSignSubmitWaitForTransaction`, a function that provides a simple way to execute the full end to end transaction submission flow. You may also leverage `generateSignSubmit`, a helper that does the same but without waiting, instead returning teh transaction hash.
- Added `fromDerivePath` to `AptosAccount`. You can use this to create an `AptosAccount` (which is a local representation of an account) using a bip44 path and mnemonics.

## 1.3.7 (2022-08-17)

- Add a transaction builder that is able to serialize transaction arguments with remote ABIs. Remote ABIs are fetchable through REST APIs. With the remote ABI transaction builder, developers can build BCS transactions by only providing the native JS values.
- Make all functions that accept `BigInt` parameters accept `BigInt | number` instead.

## 1.3.6 (2022-08-10)

- Switch back to representing certain move types (MoveModuleId, MoveStructTag, ScriptFunctionId) as strings, for both requests and responses. This reverts the change made in 1.3.2. See [#2663](https://github.com/aptos-labs/aptos-core/pull/2663) for more.
- Represent certain fields with slightly different snake casing, e.g. `ed25519_signature` now instead of `ed_25519_signature`.
- Add generated types for healthcheck endpoint.
- If the given URL is missing `/v1`, the `AptosClient` constructor will add it for you. You can opt out of this behavior by setting `doNotFixNodeUrl` to true when calling the constructor.

## 1.3.5 (2022-08-08)

- Re-expose BCS and items from `transaction_builder/builder` from the root of the module.

## 1.3.4 (2022-08-07)

- Downscaled default value for `max_gas`.

## 1.3.3 (2022-08-05)

- Update the token clients to submit transactions through BCS interface. The new token client doesn't hex-code "name", "decription" and "uri" any more. String properties are passed and saved just as strings.
- Expose `buildTransactionPayload` from ABI transaction builder. In some scenarios, developers just want to get a TransactionPayload rather than a RawTransaction.

## 1.3.2 (2022-08-04)

This special entry does not conform to the format set out by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) as there are noteworthy breaking changes with necessary rationale. Future entries will follow this format.

This release updates the SDK to work with V1 of the Aptos Node API. There are some key changes between V0 and V1 that you can read about in the [API changelog](https://github.com/aptos-labs/aptos-core/blob/main/api/doc/v1/CHANGELOG.md), refer to the notes for version 1.0.0. Accordingly, this SDK version represents breaking changes compared to 1.2.1.

- The SDK now communicates by default with the `/v1` path of the API. It will not work correctly with the v0 API. If you provide a path yourself when instantiating a client, make sure you include `/v1`, e.g. http://fullnode.devnet.aptoslabs.com/v1.
- As of this release, the API, API spec, client generated from that spec, SDK wrapper, and examples are all tested together in CI. Previously it was possible for these to be out of sync, or in some cases, they would test against a different deployment entirely, such as devnet. Now we make the guarantee that all these pieces from the same commit work together. Notably this means exactly that; there is no guarantee that the latest version of the SDK will work with a particular Aptos network, such as devnet, except for a network built from the same commit as the SDK.
- The generated client within the SDK is generated using a different tool, [openapi-typescript-codegen](https://www.npmjs.com/package/openapi-typescript-codegen). Most of these changes are transparent to the user, as we continue to wrap the generated client, but some of the generated types are different, which we mention here.
- Token types are no longer exposed from the generated client (under `Types`) as they are no longer part of the API (indeed, they never truly were). Instead you can find these definitions exposed at `TokenTypes`.
- Some functions, such as for getting account resources and events, no longer accept resource types as concatenated strings. For example:

```tsx
# Before:
const aptosCoin = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
# After
const aptosCoin = const aptosCoin = {
    address: "0x1",
    module: "coin",
    name: "CoinStore",
    generic_type_params: ["0x1::aptos_coin::AptosCoin"],
};
```

- Similarly, some endpoints no longer return this data as a string, but in a structured format, e.g. `MoveStructTag`. Remember to use something like `lodash.isEqual` to do equality checks with these structs.
- To help work with these different formats, functions for converting between them have been added to `utils`.
- A new function, `waitForTransactionWithResult`, has been added to help wait for a transaction and then get access to the response from the server once the function exits.

For help with migration, we recommend you see the updated examples under `examples/`, they demonstrate how to deal with some of these changes, such as the more structured responses. We are also available to assist in the [Aptos Discord](https://discord.com/invite/aptoslabs).

**Deprecation Notice**: On September 1st we will remove the v0 API from the running nodes. As a user of the TS SDK, the best way you can migrate prior to this is by upgrading to version 1.3.2 or higher of the SDK. We will repeatedly remind developers of this upcoming deprecation as we approach that date.

## 1.3.1 (2022-08-04)

See release notes for 1.3.2.

## 1.3.0 (2022-08-03)

See release notes for 1.3.2.

## 1.2.1 (2022-07-23)

**Note:** This entry and earlier do not conform to the format set out by [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

### Features

- Deprecate getTokenBalance api in SDK ([2ec554e](https://github.com/aptos-labs/aptos-core/commit/2ec554e6e40a81cee4e760f6f84ef7362c570240))
- Memoize chain id in aptos client ([#1589](https://github.com/aptos-labs/aptos-core/issues/1589)) ([4a6453b](https://github.com/aptos-labs/aptos-core/commit/4a6453bf0e620247557854053b661446bff807a7))
- **Multiagent:** Support multiagent transaction submission ([#1543](https://github.com/aptos-labs/aptos-core/issues/1543)) ([0f0c70e](https://github.com/aptos-labs/aptos-core/commit/0f0c70e8ed2fefa952f0c89b7edb78edc174cb49))
- Support retrieving token balance for any account ([7f93c21](https://github.com/aptos-labs/aptos-core/commit/7f93c2100f8b8e848461a0b5a395bfb76ade8667))

### Bug Fixes

- Get rid of "natual" calls ([#1678](https://github.com/aptos-labs/aptos-core/issues/1678)) ([54601f7](https://github.com/aptos-labs/aptos-core/commit/54601f79206ea0f8b8b1b0d6599d31832fc4d195))

## 1.2.0 (2022-06-28)

### Features

- Vector tests for transaction signing ([6210c10](https://github.com/aptos-labs/aptos-core/commit/6210c10d3192fd0417b35709545fae850099e4d4))
- Add royalty support for NFT tokens ([93a2cd0](https://github.com/aptos-labs/aptos-core/commit/93a2cd0bfd644725ac524f419e94077e0b16343b))
- Add transaction builder examples ([a710a50](https://github.com/aptos-labs/aptos-core/commit/a710a50e8177258d9c0766762b3c2959fc231259))
- Support transaction simulation ([93073bf](https://github.com/aptos-labs/aptos-core/commit/93073bf1b508d00cfa1f8bb441ed57085fd08a82))

### Bug Fixes

- Fix a typo, natual now becomes natural ([1b7d295](https://github.com/aptos-labs/aptos-core/commit/1b7d2957b79a5d2821ada0c5096cf43c412e0c2d)), closes [#1526](https://github.com/aptos-labs/aptos-core/issues/1526)
- Fix Javascript example ([5781fee](https://github.com/aptos-labs/aptos-core/commit/5781fee74b8f2b065e7f04c2f76952026860751d)), closes [#1405](https://github.com/aptos-labs/aptos-core/issues/1405)
