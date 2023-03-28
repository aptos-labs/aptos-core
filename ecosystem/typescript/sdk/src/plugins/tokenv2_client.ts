// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable max-len */

import { AptosAccount } from "../account/aptos_account";
import { AnyNumber, Bytes } from "../bcs";
import { MAX_U64_BIG_INT } from "../bcs/consts";
import { AptosClient, OptionalTransactionArgs } from "../providers/aptos_client";
import { TransactionBuilderRemoteABI } from "../transaction_builder";

/**
 * Class for creating, minting and managing minting NFT collections and tokens
 */
export class TokenV2Client {
  aptosClient: AptosClient;

  /**
   * Creates new TokenClient instance
   *
   * @param aptosClient AptosClient instance
   */
  constructor(aptosClient: AptosClient) {
    this.aptosClient = aptosClient;
  }

  /**
   *
   */
  async createCollection(
    account: AptosAccount,
    description: string,
    maxSupply: AnyNumber = MAX_U64_BIG_INT,
    name: string,
    uri: string,
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
  ) {
    const builder = new TransactionBuilderRemoteABI(this.aptosClient, { sender: account.address(), ...extraArgs });
    const rawTxn = await builder.build(
      "0x423eab63bed73bb1febb3630e803a1d18b518e798cd2a28d4fea8ba53d097cb1::aptos_token::create_collection",
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
    const pendingTransaction = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  /**
   *
   */
  async mint(
    account: AptosAccount,
    collection: string,
    description: string,
    name: string,
    uri: string,
    propertyKeys: Array<string> = [],
    propertyTypes: Array<string> = [],
    propertyValues: Array<Bytes> = [],
    extraArgs?: OptionalTransactionArgs,
  ) {
    const builder = new TransactionBuilderRemoteABI(this.aptosClient, { sender: account.address(), ...extraArgs });
    const rawTxn = await builder.build(
      "0x423eab63bed73bb1febb3630e803a1d18b518e798cd2a28d4fea8ba53d097cb1::aptos_token::mint",
      [],
      [collection, description, name, uri, propertyKeys, propertyTypes, propertyValues],
    );
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  async mintSoulBound(
    account: AptosAccount,
    collection: string,
    description: string,
    name: string,
    uri: string,
    property_keys: string[],
    property_types: string[],
    property_values: number[][],
    soul_bound_to: string,
    extraArgs?: OptionalTransactionArgs,
  ) {
    const builder = new TransactionBuilderRemoteABI(this.aptosClient, { sender: account.address(), ...extraArgs });
    const rawTxn = await builder.build(
      "0x423eab63bed73bb1febb3630e803a1d18b518e798cd2a28d4fea8ba53d097cb1::aptos_token::mint_soul_bound",
      [],
      [collection, description, name, uri, property_keys, property_types, property_values, soul_bound_to, extraArgs],
    );
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  // async burn(account:AptosAccount,token:Object<T>){

  // }
}
