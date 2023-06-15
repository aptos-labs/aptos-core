import { AptosAccount } from "../account";
import { HexString } from "./hex_string";

export interface PaginationArgs {
  start?: AnyNumber;
  limit?: number;
}
export type MaybeHexString = HexString | string;

export interface OptionalTransactionArgs {
  maxGasAmount?: Uint64;
  gasUnitPrice?: Uint64;
  expireTimestamp?: Uint64;
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

export type Seq<T> = T[];

export type Uint8 = number;
export type Uint16 = number;
export type Uint32 = number;
export type Uint64 = bigint;
export type Uint128 = bigint;
export type Uint256 = bigint;
export type AnyNumber = bigint | number;
export type Bytes = Uint8Array;
