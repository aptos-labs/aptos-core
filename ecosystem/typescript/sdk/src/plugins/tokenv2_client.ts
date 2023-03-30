// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable max-len */

import { AptosAccount } from "../account/aptos_account";
import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { AnyNumber, bcsToBytes } from "../bcs";
import { MAX_U64_BIG_INT } from "../bcs/consts";
import { AptosClient, OptionalTransactionArgs } from "../providers/aptos_client";
import { TransactionBuilderRemoteABI } from "../transaction_builder";
import { getPropertyValueRaw } from "../utils/property_map_serde";
import { HexString } from "../utils";
import { AccountAddress } from "../aptos_types";

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
    propertyValues: Array<string> = [],
    extraArgs?: OptionalTransactionArgs,
  ) {
    const builder = new TransactionBuilderRemoteABI(this.aptosClient, { sender: account.address(), ...extraArgs });
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
    const pendingTransaction = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);
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
    const builder = new TransactionBuilderRemoteABI(this.aptosClient, { sender: account.address(), ...extraArgs });
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
    const pendingTransaction = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  /**
   *
   */
  async burn(account: AptosAccount, token: string, extraArgs?: OptionalTransactionArgs) {
    const builder = new TransactionBuilderRemoteABI(this.aptosClient, { sender: account.address(), ...extraArgs });
    const rawTxn = await builder.build("0x4::aptos_token::burn", ["0x4::token::Token"], [token]);
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  /**
   * In token v2:
   * Collection object addresses are generated as sha256 hash of (creator address + collection_name)
   */

  collectionObjectAddress(creator: AptosAccount, collectionName: string): HexString {
    const source = bcsToBytes(AccountAddress.fromHex(creator.address()));
    const seed = new TextEncoder().encode(collectionName);

    const bytes = new Uint8Array([...source, ...seed, 254]);

    const hash = sha3Hash.create();
    hash.update(bytes);

    return HexString.fromUint8Array(hash.digest());
  }

  /**
   * Token object addresses are generated as sha256 hash of (creator address + collection's name + :: + token name)
   */
  tokenObjectAddress(creator: AptosAccount, collectionName: string, tokenName: string): HexString {
    const source = bcsToBytes(AccountAddress.fromHex(creator.address()));
    const collectionBytes = new TextEncoder().encode(collectionName);
    const tokenBytes = new TextEncoder().encode(tokenName);

    const seed = new Uint8Array(collectionBytes.length + tokenBytes.length + 2);
    seed.set(collectionBytes);
    seed.set([58, 58], collectionBytes.length);
    seed.set(tokenBytes, collectionBytes.length + 2);

    const bytes = new Uint8Array([...source, ...seed, 254]);

    const hash = sha3Hash.create();
    hash.update(bytes);

    return HexString.fromUint8Array(hash.digest());
  }
}
