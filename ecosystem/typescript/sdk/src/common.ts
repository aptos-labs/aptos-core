// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { HexString, MaybeHexString } from "./hex_string";
import * as Gen from "./generated/index";

// There are places that an instance of `AptosClient` is needed. However, depending on `AptosClient`
// directly might create a circular dependency. In such cases, `IAptosClient` can be used.
export abstract class IAptosClient {
  abstract getAccountModules(accountAddress: MaybeHexString): Promise<Gen.MoveModuleBytecode[]>;

  abstract getAccount(accountAddress: MaybeHexString): Promise<Gen.AccountData>;

  abstract getChainId(): Promise<number>;

  abstract lookupOriginalAddress(addressOrAuthKey: MaybeHexString): Promise<HexString>;
}
