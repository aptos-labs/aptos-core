// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable max-len */

import { AptosAccount } from "../account/aptos_account";
import { AnyNumber } from "../bcs";
import { MAX_U64_BIG_INT } from "../bcs/consts";
import { Provider } from "../providers";
import { AptosClient, OptionalTransactionArgs } from "../providers/aptos_client";
import { TransactionBuilderRemoteABI } from "../transaction_builder";
import { getPropertyValueRaw } from "../utils/property_map_serde";

/**
 * Class for managing aptos_token
 */
export class AptosToken {
  provider: Provider;

  /**
   * Creates new AptosToken instance
   *
   * @param provider Provider instance
   */
  constructor(provider: Provider) {
    this.provider = provider;
  }

  /**
   * Creates a new collection within the specified account
   *
   * @param account AptosAccount where collection will be created
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
    account: AptosAccount,
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
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x4::aptos_token::create_collection",
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
    );
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
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
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x4::aptos_token::mint",
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
    );
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
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
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x4::aptos_token::mint_soul_bound",
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
    );
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }
}
