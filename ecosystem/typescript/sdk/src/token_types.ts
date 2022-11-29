// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { deserializePropertyMap, PropertyMap } from "./utils/property_map_serde";

export class TokenData {
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

  /** default properties of token data */
  default_properties: PropertyMap;

  /** mutability config of tokendata fields */
  mutability_config: boolean[];

  constructor(
    collection: string,
    description: string,
    name: string,
    maximum: number,
    supply: number,
    uri: string,
    default_properties: any,
    mutability_config: boolean[],
  ) {
    this.collection = collection;
    this.description = description;
    this.name = name;
    this.maximum = maximum;
    this.supply = supply;
    this.uri = uri;
    this.default_properties = deserializePropertyMap(default_properties);
    this.mutability_config = mutability_config;
  }
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

export class Token {
  id: TokenId;

  /** server will return string for u64 */
  amount: U64;

  /** the property map of the token */
  token_properties: PropertyMap;

  constructor(id: TokenId, amount: U64, token_properties: any) {
    this.id = id;
    this.amount = amount;
    this.token_properties = deserializePropertyMap(token_properties);
  }
}
