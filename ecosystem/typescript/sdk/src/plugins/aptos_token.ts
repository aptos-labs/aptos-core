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

/**
 * Class for managing aptos_token
 */
export class AptosToken {
  provider: Provider;

  tokenType: string = "0x4::token::Token";

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
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(`0x4::aptos_token::${funcName}`, typeArgs, args);
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  /**
   * Creates a new collection within the specified account
   *
   * @param creator AptosAccount where collection will be created
   * @param description Collection description
   * @param name Collection name
   * @param uri URL to additional info about collection
   * @param maxSupply Maximum number of `token_data` allowed within this collection
   * @param royaltyNumerator The numerator for calculating royalty
   * @param royaltyDenominator The denominator for calculating royalty
   * @param mutableDescription Whether the description in mutable
   * @param mutableRoyalty Whether the royalt in mutable
   * @param mutableURI Whether the URI in mutable
   * @param mutableTokenDescription Whether the token description in mutable
   * @param mutableTokenName Whether the token name in mutable
   * @param mutableTokenProperties Whether the token properties are mutable
   * @param mutableTokenURI Whether the token URI in mutable
   * @param tokensBurnableByCreator Whether token burnable by creator
   * @param tokensFreezableByCreator Whether token freezable by creator
   * @returns The hash of the transaction submitted to the API
   */
  async createCollection(
    creator: AptosAccount,
    description: string,
    name: string,
    uri: string,
    maxSupply: AnyNumber = MAX_U64_BIG_INT,
    royaltyNumerator: number = 0,
    royaltyDenominator: number = 0,
    mutableDescription: boolean = true,
    mutableRoyalty: boolean = true,
    mutableURI: boolean = true,
    mutableTokenDescription: boolean = true,
    mutableTokenName: boolean = true,
    mutableTokenProperties: boolean = true,
    mutableTokenURI: boolean = true,
    tokensBurnableByCreator: boolean = true,
    tokensFreezableByCreator: boolean = true,
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
        mutableDescription,
        mutableRoyalty,
        mutableURI,
        mutableTokenDescription,
        mutableTokenName,
        mutableTokenProperties,
        mutableTokenURI,
        tokensBurnableByCreator,
        tokensFreezableByCreator,
        royaltyNumerator,
        royaltyDenominator,
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
    key: string,
    type: string,
    value: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "add_property",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), key, type, getSinglePropertyValueRaw(value, type)],
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
    key: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "remove_property",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), key],
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
    key: string,
    type: string,
    value: string,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    return this.submitTransaction(
      creator,
      "update_property",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), key, type, getSinglePropertyValueRaw(value, type)],
      extraArgs,
    );
  }

  /**
   * Transfer a token ownership
   *
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

  /**
   * Transfer a token amount from the sender primary_store to the recipient primary_store
   *
   * @param sender The sender account
   * @param token The token address - For example if you’re transferring USDT, this would be the USDT address
   * @param recipient Recipient primary wallet address
   * @returns The hash of the transaction submitted to the API
   */
  async transferTokenAmount(
    sender: AptosAccount,
    token: MaybeHexString,
    recipient: MaybeHexString,
    amount: number = 0,
    tokenType?: string,
    extraArgs?: OptionalTransactionArgs,
  ) {
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: sender.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x1::primary_store::transfer",
      [tokenType || this.tokenType],
      [HexString.ensure(token).hex(), HexString.ensure(recipient).hex(), amount],
    );
    const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }
}
