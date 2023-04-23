// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable max-len */

import { AptosAccount } from "../account/aptos_account";
import { AnyNumber } from "../bcs";
import { MAX_U64_BIG_INT } from "../bcs/consts";
import { Provider } from "../providers";
import { AptosClient, OptionalTransactionArgs } from "../providers/aptos_client";
import { TransactionBuilderRemoteABI } from "../transaction_builder";
import { HexString, MaybeHexString } from "../utils";
import { getPropertyValueRaw, getSinglePropertyValueRaw } from "../utils/property_map_serde";

export interface CreateCollectionOptions {
  royaltyNumerator?: number;
  royaltyDenominator?: number;
  mutableDescription?: boolean;
  mutableRoyalty?: boolean;
  mutableURI?: boolean;
  mutableTokenDescription?: boolean;
  mutableTokenName?: boolean;
  mutableTokenProperties?: boolean;
  mutableTokenURI?: boolean;
  tokensBurnableByCreator?: boolean;
  tokensFreezableByCreator?: boolean;
}

const PropertyTypeMap = {
  BOOLEAN: "bool",
  U8: "u8",
  U16: "u16",
  U32: "u32",
  U64: "u64",
  U128: "u128",
  U256: "u256",
  ADDRESS: "address",
  VECTOR: "vector<u8>",
  STRING: "string",
};

export type PropertyType = keyof typeof PropertyTypeMap;

/**
 * Class for managing aptos_token
 */
export class AptosToken {
  readonly provider: Provider;

  private readonly tokenType: string = "0x4::token::Token";

  /**
   * Creates new AptosToken instance
   *
   * @param provider Provider instance
   */
  constructor(provider: Provider) {
    this.provider = provider;
  }

  private async submitTransaction(
    account: AptosAccount,
    funcName: string,
    typeArgs: string[],
    args: any[],
    extraArgs?: OptionalTransactionArgs,
  ) {
    const builder = new TransactionBuilderRemoteABI(this.provider, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(`0x4::aptos_token::${funcName}`, typeArgs, args);
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  /**
   * Creates a new collection within the specified account
   *
   * @param creator AptosAccount where collection will be created
   * @param description Collection description
   * @param name Collection name
   * @param uri URL to additional info about collection
   * @param options CreateCollectionOptions type. By default all values set to `true` or `0`
   * @returns The hash of the transaction submitted to the API
   */
  async createCollection(
    creator: AptosAccount,
    description: string,
    name: string,
    uri: string,
    maxSupply: AnyNumber = MAX_U64_BIG_INT,
    options?: CreateCollectionOptions,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "create_collection",
      [],
      [
        description,
        maxSupply,
        name,
        uri,
        options?.mutableDescription ?? true,
        options?.mutableRoyalty ?? true,
        options?.mutableURI ?? true,
        options?.mutableTokenDescription ?? true,
        options?.mutableTokenName ?? true,
        options?.mutableTokenProperties ?? true,
        options?.mutableTokenURI ?? true,
        options?.tokensBurnableByCreator ?? true,
        options?.tokensFreezableByCreator ?? true,
        options?.royaltyNumerator ?? 0,
        options?.royaltyDenominator ?? 0,
      ],
      extraArgs,
    );
  }

  /**
   * Mint a new token within the specified account
   *
   * @param account AptosAccount where token will be created
   * @param collection Name of collection, that token belongs to
   * @param description Token description
   * @param name Token name
   * @param uri URL to additional info about token
   * @param propertyKeys the property keys for storing on-chain properties
   * @param propertyTypes the type of property values
   * @param propertyValues the property values to be stored on-chain
   * @returns The hash of the transaction submitted to the API
   */
  async mint(
    account: AptosAccount,
    collection: string,
    description: string,
    name: string,
    uri: string,
    propertyKeys: Array<string> = [],
    propertyTypes: Array<string> = [],
    propertyValues: Array<string> = [],
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      account,
      "mint",
      [],
      [
        collection,
        description,
        name,
        uri,
        propertyKeys,
        propertyTypes,
        getPropertyValueRaw(propertyValues, propertyTypes),
      ],
      extraArgs,
    );
  }

  /**
   * Mint a soul bound token into a recipient's account
   *
   * @param account AptosAccount that mints the token
   * @param collection Name of collection, that token belongs to
   * @param description Token description
   * @param name Token name
   * @param uri URL to additional info about token
   * @param recipient AptosAccount where token will be created
   * @param propertyKeys the property keys for storing on-chain properties
   * @param propertyTypes the type of property values
   * @param propertyValues the property values to be stored on-chain
   * @returns The hash of the transaction submitted to the API
   */
  async mintSoulBound(
    account: AptosAccount,
    collection: string,
    description: string,
    name: string,
    uri: string,
    recipient: AptosAccount,
    propertyKeys: Array<string> = [],
    propertyTypes: Array<string> = [],
    propertyValues: Array<string> = [],
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      account,
      "mint_soul_bound",
      [],
      [
        collection,
        description,
        name,
        uri,
        propertyKeys,
        propertyTypes,
        getPropertyValueRaw(propertyValues, propertyTypes),
        recipient.address().hex(),
      ],
      extraArgs,
    );
  }

  /**
   * Burn a token by its creator
   * @param creator Creator account
   * @param token Token address
   * @returns The hash of the transaction submitted to the API
   */
  async burnToken(
    creator: AptosAccount,
    token: MaybeHexString,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "burn",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex()],
      extraArgs,
    );
  }

  /**
   * Freeze token transfer ability
   * @param creator Creator account
   * @param token Token address
   * @returns The hash of the transaction submitted to the API
   */
  async freezeTokenTransafer(
    creator: AptosAccount,
    token: MaybeHexString,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "freeze_transfer",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex()],
      extraArgs,
    );
  }

  /**
   * Unfreeze token transfer ability
   * @param creator Creator account
   * @param token Token address
   * @returns The hash of the transaction submitted to the API
   */
  async unfreezeTokenTransafer(
    creator: AptosAccount,
    token: MaybeHexString,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "unfreeze_transfer",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex()],
      extraArgs,
    );
  }

  /**
   * Set token description
   * @param creator Creator account
   * @param token Token address
   * @param description Token description
   * @returns The hash of the transaction submitted to the API
   */
  async setTokenDescription(
    creator: AptosAccount,
    token: MaybeHexString,
    description: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "set_description",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), description],
      extraArgs,
    );
  }

  /**
   * Set token name
   * @param creator Creator account
   * @param token Token address
   * @param name Token name
   * @returns The hash of the transaction submitted to the API
   */
  async setTokenName(
    creator: AptosAccount,
    token: MaybeHexString,
    name: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "set_name",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), name],
      extraArgs,
    );
  }

  /**
   * Set token URI
   * @param creator Creator account
   * @param token Token address
   * @param uri Token uri
   * @returns The hash of the transaction submitted to the API
   */
  async setTokenURI(
    creator: AptosAccount,
    token: MaybeHexString,
    uri: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "set_uri",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), uri],
      extraArgs,
    );
  }

  /**
   * Add token property
   * @param creator Creator account
   * @param token Token address
   * @param key the property key for storing on-chain property
   * @param type the type of property value
   * @param value the property value to be stored on-chain
   * @returns The hash of the transaction submitted to the API
   */
  async addTokenProperty(
    creator: AptosAccount,
    token: MaybeHexString,
    propertyKey: string,
    propertyType: PropertyType,
    propertyValue: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "add_property",
      [tokenType || this.tokenType],
      [
        HexString.ensure(token).hex(),
        propertyKey,
        PropertyTypeMap[propertyType],
        getSinglePropertyValueRaw(propertyValue, PropertyTypeMap[propertyType]),
      ],
      extraArgs,
    );
  }

  /**
   * Remove token property
   * @param creator Creator account
   * @param token Token address
   * @param key the property key stored on-chain
   * @returns The hash of the transaction submitted to the API
   */
  async removeTokenProperty(
    creator: AptosAccount,
    token: MaybeHexString,
    propertyKey: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "remove_property",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), propertyKey],
      extraArgs,
    );
  }

  /**
   * Update token property
   * @param creator Creator account
   * @param token Token address
   * @param key the property key stored on-chain
   * @param type the property typed stored on-chain
   * @param value the property value to be stored on-chain
   * @returns The hash of the transaction submitted to the API
   */
  async updateTokenProperty(
    creator: AptosAccount,
    token: MaybeHexString,
    propertyKey: string,
    propertyType: PropertyType,
    propertyValue: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "update_property",
      [tokenType || this.tokenType],
      [
        HexString.ensure(token).hex(),
        propertyKey,
        PropertyTypeMap[propertyType],
        getSinglePropertyValueRaw(propertyValue, PropertyTypeMap[propertyType]),
      ],
      extraArgs,
    );
  }

  async addTypedProperty(
    creator: AptosAccount,
    token: MaybeHexString,
    propertyKey: string,
    propertyType: PropertyType,
    propertyValue: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ) {
    return this.submitTransaction(
      creator,
      "add_typed_property",
      [tokenType || this.tokenType, PropertyTypeMap[propertyType]],
      [HexString.ensure(token).hex(), propertyKey, propertyValue],
      extraArgs,
    );
  }

  async updateTypedProperty(
    creator: AptosAccount,
    token: MaybeHexString,
    propertyKey: string,
    propertyType: PropertyType,
    propertyValue: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ) {
    return this.submitTransaction(
      creator,
      "update_typed_property",
      [tokenType || this.tokenType, PropertyTypeMap[propertyType]],
      [HexString.ensure(token).hex(), propertyKey, propertyValue],
      extraArgs,
    );
  }

  /**
   * Transfer a token ownership.
   * We can transfer a token only when the token is not frozen (i.e. owner transfer is not disabled such as for soul bound tokens)
   * @param owner The account of the current token owner
   * @param token Token address
   * @param recipient Recipient address
   * @returns The hash of the transaction submitted to the API
   */
  async transferTokenOwnership(
    owner: AptosAccount,
    token: MaybeHexString,
    recipient: MaybeHexString,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: owner.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x1::object::transfer",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), HexString.ensure(recipient).hex()],
    );
    const bcsTxn = AptosClient.generateBCSTransaction(owner, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }
}
