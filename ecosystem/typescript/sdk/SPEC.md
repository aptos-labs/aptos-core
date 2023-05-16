# Aptos SDK Specification Document

[Table of Content](#table-of-content)

![Status](https://img.shields.io/badge/version-1.0-brightgreen.svg)

## Overview

The goal of this document is to set a shared standard for implementation and development of all Aptos SDKs.

This is a living document, and should be changed and updated as changes are made when developer needs are discovered.

### Requirement Prioritization

The following document follows the [MoSCoW](https://en.wikipedia.org/wiki/MoSCoW_method) method of prioritising rules. Please follow the following guidelines when evaluating rules.

- `MUST` - Rules labeled as **must** are requirements that should not be deviated from at any cost
- `SHOULD` - Rules labeled as **should** are requirements that could be deviated from if needed, though this will have to be documented and cleared with all stakeholders before it can be disregarded.
- `COULD` - Rules labeled as **could** are requirements that are desirable but not necessary and therefore would be nice to have where time and resources permit.

We do not use the fourth **`won't`** level in this specification.

## Table of Contents

- Base Requirements
  - [1. BCS Encoding & Decoding](#1-BCS-Encoding--Decoding)
  - [2. API servers](#2-API-servers)
  - [3. Account Management](#3-Account-Management)
  - [4. Transaction](#4-Transaction)
  - [5. Coin Management](#5-Coin-Management)
- Maintenance Requirements
  - [6. Source Control](#6-source-control)
  - [7. Releases & Versioning](#7-releases--versioning)
  - [8. CI Server](#8-ci-server)
- Additional Content Requirements
  - [9. Documentation](#9-documentation)
- Dependencies & Infrastructure Requirements
  - [10. Testing](#10-testing)
  - [11. Dependencies](#11-dependencies)
  - [12. HTTP Client](#12-http-client)
  - [13. Logging](#13-reporting)
  - [14. Reporting](#14-reporting)
- Initialization & Interaction Requirements
  - [15. Initialization](#15-initalization)
  - [16. Error Handling](#16-error-handling)

## Base Requirements

### 1. BCS Encoding & Decoding

- [ ] **1.1** The SDK **must** support BCS encoding & decoding for.
  - [ ] **1.1.1** Unsigned integers u8, u32, u64, u128, u256.
  - [ ] **1.1.2** Optional arguments.
  - [ ] **1.1.3** Tuples.
  - [ ] **1.1.4** Enums.
  - [ ] **1.1.5** UTF-8 Strings as vector of u8.
  - [ ] **1.1.6** Structs.
  - [ ] **1.1.7** Vectors using uleb128.
  - [ ] **1.1.8** Addresses as a fixed 32 byte type.
  - [ ] **1.1.9** Generics.
  - [ ] **1.1.10** bool.
  - [ ] **1.1.11** Object<T> encodings as an address.
- [ ] **1.2** The encoding & decoding details **should** be hidden from the end user and done by the SDK.
  - [ ] **1.2.1** The SDK **must** encode each piece in order.
  - [ ] **1.2.2** The SDK **should** encode based on a given struct.

### 2. API servers

- [ ] **2.1** The SDK **must** interact directly with the Aptos REST API.
  - [ ] **2.1.1** The SDK **must** support pagination with optional `start` and `limit` parameters to the REST API request.
  - [ ] **2.1.2** The SDK **should** implement a client that adheres to the OpenAPI spec https://fullnode.mainnet.aptoslabs.com/v1/spec#/.
- [ ] **2.2** The SDK **should** interact directly with the Aptos Indexer API.
  - [ ] **2.2.1** The SDK **must** support pagination with optional `offset` and `limit` parameters to Indexer API request.
  - [ ] **2.2.2** The SDK **must** validate the account address when interacting with the Indexer API.
    - [ ] **2.2.2.1** The SDK **must** validate an account address is a 64 character hex string with a leading `0x`.
  - [ ] **2.2.3** The SDK **must** handle premade GraphQL queries https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql.
  - [ ] **2.2.3** The SDK **should** allow the user to pass in a custom GraphQL query.
- [ ] **2.3** The SDK **should** support interactions with Aptos devnet, testnet and mainnet networks.
- [ ] **2.4** The SDK **should** support interactions with a custom URL.

### 3. Account Management

- [ ] **3.1** The SDK **must** support account creation.
- [ ] **3.2** The SDK **must** support account APT coin balance read.
- [ ] **3.3** The SDK **must** support key management for Ed25519 keys.
  - [ ] **3.3.1** The SDK **must** support Key generation
  - [ ] **3.3.2** The SDK **must** support Key loading from a file
  - [ ] **3.3.3** The SDK **must** support Key loading from a byte array
  - [ ] **3.3.4** The SDK **should** support Key rotation
  - [ ] **3.3.5** The SDK **could** Key management via local storage
  - [ ] **3.3.6** The SDK **could** Key management via hardware wallet (e.g. Ledger, Keystone, etc.)
- [ ] **3.4** The SDK **must** support local single-threaded sequence number management.
  - [ ] **3.4.1** The SDK **should** support multi-account sequence number management
  - [ ] **3.4.2** The SDK **should** support single threaded sequence number high throughput smart queuing
- [ ] **3.5** The SDK **must** provide multi-agent signer support.
- [ ] **3.6** The SDK **should** support account any coin balance read.
- [ ] **3.7** The SDK **should** provide multi-Ed25519 signer support.
- [ ] **3.8** The SDK **should** provide mnemonic support.
- [ ] **3.9** The SDK **should** provide onchain multi-sig support.
- [ ] **3.10** The SDK **should** provide resource account support.
- [ ] **3.10** The SDK **should** provide Object support for deriving known objects and object ownership.

### 4. Transaction

- [ ] **4.1** The SDK **must** support entry function payload transaction submission.
- [ ] **4.2** The SDK **must** support script payload transaction submission.
- [ ] **4.3** The SDK **must** support simulation of transactions.
- [ ] **4.4** The SDK **must** support view function payload.
- [ ] **4.5** The SDK **should** use the gas estimation API to determine the gas price when building transaction payloads.
  - [ ] **4.5.1** The SDK **must** cache the response from the gas estimation API for a set period of time (e.g. 1 minute).

### 5. Coin Management

- [ ] **5.1** The SDK **must** support APT coin transfer.
- [ ] **5.1** The SDK **should** support other coin transfer.

## Maintenance Requirements

### 6. Source Control

- [ ] **6.1** The source code for the SDK **must** be maintained within Git version control.
- [ ] **6.2** The source code **must** be hosted publicly.
- [ ] **6.3** Development of new features **should** happen on feature branches.
- [ ] **6.4** Feature branches **should** pass all tests and linting before they can be merged into the `main` branch.
- [ ] **6.5** Source control **should** contain tags for each release of the SDK.
- [ ] **6.6** The `main` branch **should** be kept in a condition that allows for direct use through checkout.
- [ ] **6.7** The source code **should** use GitHub for public hosting.

### 7. Releases & Versioning

- [ ] **7.1** The SDK **must** use [Semantic Versioning](http://semver.org/) to increment the version number as changes are made.
- [ ] **7.2** For every new release the `CHANGELOG` file **must** to be updated with the `Major`, `Minor` and `Patch` changes.
- [ ] **7.3** A release package **must** include the documentation `README` file.
- [ ] **7.4** A release package **must** include the `LICENSE` file.
- [ ] **7.5** A release package **must** include the `CHANGELOG` file.

- [ ] **7.6** A release package **should** not include unnecessary source code files or intermediary files for the SDK.

- [ ] **7.7** The name of the SDK **should** follow language best practices, and be one of `aptos` or `Aptos`.
- [ ] **7.8** If the preferred name of the SDK is not available, it **could** be one of `aptos-sdk`, `AptosSDK`, or `aptosdev`.
- [ ] **7.9** As soon as the first public version of the library has been signed off, the version **should** be bumped to `1.0.0`.
- [ ] **7.10** New releases **could** be deployed automatically to the package manager using the CI server.

### 8. CI Server

- [ ] **8.1** A Continuous Integration (CI) server **must** be used to automatically test any branch of the Git repository.
- [ ] **8.2** The CI server **could** test on different platforms, including Windows, Linux, and macOS.
- [ ] **8.3** The CI server **could** test new Git tags, and build and push the package to the package manager.

## Content Requirements

### 9. Documentation

- [ ] **9.1** The SDK **must** include a `README` file.

  - [ ] **9.1.1** The `README` file **must** have instructions on how to install the SDK using a package manager.
  - [ ] **9.1.2** The `README` file **must** link to the `LICENSE` file.
  - [ ] **9.1.3** The `README` file **must** document any installation requirements and prerequisites.
  - [ ] **9.1.4** The `README` file **should** be written in Markdown.
  - [ ] **9.1.5** The `README` file **should** include a version badge.
  - [ ] **9.1.6** The `README` file **should** include a test status badge.
  - [ ] **9.1.7** The `README` file **should** document all the different ways the SDK can be initialized.
  - [ ] **9.1.8** The `README` file **should** include official support channels.
  - [ ] **9.1.9** The `README` file **could** have instructions on how to install the SDK from version control.

- [ ] **9.2** The SDK **must** include a `CHANGELOG` file.
- [ ] **9.3** The SDK **must** include a `CODE_OF_CONDUCT` file.
- [ ] **9.4** The SDK **must** include a `CONTRIBUTING` file.
  - [ ] **9.4.1** The Contribution Guidelines **should** include instructions on how to run the SDK in development/testing mode.
- [ ] **9.5** The SDK **should** include a `ISSUE_TEMPLATE` file.
- [ ] **9.6** The SDK **should** include a `PULL_REQUEST_TEMPLATE` file.
- [ ] **9.7** The SDK **should** include a `SUPPORT` file.
- [ ] **9.8** The GitHub repository **should** have a title in the format, e.g. "Typescript library for the Aptos network"
- [ ] **9.9** The GitHub repository **should** have the following tags: `aptos`, `blockchain`, `web3`, `sdk`, `library`.

## Dependencies & Infrastructure Requirements

### 10. Testing

- [ ] **10.1** The SDK **must** be thoroughly tested.
- [ ] **10.2** For any real API calls, the tests **must** use the Aptos testnet or devnet network.
- [ ] **10.3** The tests **should** have integration tests to make the network calls.
- [ ] **10.4** The tests **should** test responses.

### 11. Dependencies

- [ ] **11.1** The SDK **must** limit its runtime dependencies.
- [ ] **11.2** The SDK **should** have no runtime dependencies.
- [ ] **11.3** The SDK **could** use any amount of development and test dependencies.

### 12. HTTP Client

- [ ] **12.1** The SDK **must** use a well supported HTTP client.
- [ ] **12.2** The SDK **should** use a HTTP2 supported client.
- [ ] **12.3** A HTTP client from the standard libraries **should** be used.
- [ ] **12.4** The SDK **could** allow a developer to provide an alternative HTTP client.

### 13. Logging

- [ ] **13.1** The SDK **must** be able to log activities to a logger.
- [ ] **13.2** The logger **must** allow enabling/disabling of debug mode.
- [ ] **13.3** The logger **should** use the default runtime log.
- [ ] **13.4** The logger **should** allow a developer to provide an alternative logger.
- [ ] **13.5** When debugging is enabled, the logger **should** log (and only log) the request object, response object, and optionally any raw HTTP response object of no response object could be formed.

### 14. Reporting

- [ ] **14.1** The SDK **should** pass a custom header `x-aptos-client` with the format `<sdk-id>/<sdk-version>`.
  - Example with known sdk version: `aptos-ts-sdk/1.8.4`

## Initialization & Interaction Requirements

### 15. Initialization

- [ ] **15.1** The SDK client **must** allow selection of the base URL by name (`devnet` , `tesnet` , `mainnet`).
- [ ] **15.2** The SDK client **must** allow for setting a custom base URL directly (e.g `http://localhost:8080`).

### 16. Error Handling

- [ ] **16.1** The SDK **should** raise a `ApiError` for any request that did not result a HTTP 200 or 201 response code.
- [ ] **16.4** The response error object **should** contain a `message` attribute containing the message of the error, e.g. `account_not_found`.
- [ ] **16.5** The response error object **should** contain a `error_code` attribute containing the error code, e.g. `404`.
- [ ] **16.5** The response error object **should** contain a `vm_error_code` attribute containing the vm error code, e.g. `0`.
