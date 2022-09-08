// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export interface TokenData {
  /** Unique name within this creator's account for this Token's collection */
  collection: string;

  /** Description of Token */
  description: string;

  /** Name of Token */
  name: string;

  /** Optional maximum number of this Token */
  maximum?: number;

  /** Total number of this type of Token */
  supply: number;

  /** URL for additional information / media */
  uri: string;
}

export interface TokenDataId {
  /** Token creator address */
  creator: string;

  /** Unique name within this creator's account for this Token's collection */
  collection: string;

  /** Name of Token */
  name: string;
}

export interface TokenId {
  token_data_id: TokenDataId;

  /** version number of the property map */
  property_version: string;
}

/** server will return string for u64 */
type U64 = string;

export interface Token {
  id: TokenId;
  /** server will return string for u64 */
  amount: U64;
}
