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
    propertyValues: Array<string> = [],
    extraArgs?: OptionalTransactionArgs,
  ) {
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

  async mintSoulBound(
    account: AptosAccount,
    collection: string,
    description: string,
    name: string,
    uri: string,
    propertyKeys: Array<string>,
    propertyTypes: Array<string>,
    propertyValues: Array<string>,
    soulBoundTo: AptosAccount,
    extraArgs?: OptionalTransactionArgs,
  ) {
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
        soulBoundTo.address().hex(),
      ],
    );
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }
}
