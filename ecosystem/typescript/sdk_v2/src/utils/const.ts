// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { Uint128, Uint16, Uint32, Uint64, Uint8, Uint256 } from "../types";
import { Network } from "./api-endpoints";

export const DEFAULT_NETWORK = Network.DEVNET;

export enum AptosApiType {
  FULLNODE,
  INDEXER,
  FAUCET,
}

// Upper bound values for uint8, uint16, uint64 and uint128
export const MAX_U8_NUMBER: Uint8 = 2 ** 8 - 1;
export const MAX_U16_NUMBER: Uint16 = 2 ** 16 - 1;
export const MAX_U32_NUMBER: Uint32 = 2 ** 32 - 1;
export const MAX_U64_BIG_INT: Uint64 = BigInt(2) ** BigInt(64) - BigInt(1);
export const MAX_U128_BIG_INT: Uint128 = BigInt(2) ** BigInt(128) - BigInt(1);
export const MAX_U256_BIG_INT: Uint256 = BigInt(2) ** BigInt(256) - BigInt(1);
