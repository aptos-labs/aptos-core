// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import * as TokenTypes from "./token_types";
import * as Gen from "./generated/index";
import { HexString, MaybeHexString } from "./hex_string";
import { BCS, TxnBuilderTypes, TransactionBuilderABI } from "./transaction_builder";
import { MAX_U64_BIG_INT } from "./transaction_builder/bcs/consts";
import { TOKEN_ABIS } from "./abis";

/**
 * Class for creating, minting and managing minting NFT collections and tokens
 */
export class TokenClient {
  aptosClient: AptosClient;

  transactionBuilder: TransactionBuilderABI;

  /**
   * Creates new TokenClient instance
   * @param aptosClient AptosClient instance
   */
  constructor(aptosClient: AptosClient) {
    this.aptosClient = aptosClient;
    this.transactionBuilder = new TransactionBuilderABI(TOKEN_ABIS.map((abi) => new HexString(abi).toUint8Array()));
  }

  /**
   * Brings together methods for generating, signing and submitting transaction
   * @param account AptosAccount which will sign a transaction
   * @param payload Transaction payload. It depends on transaction type you want to send
   * @returns Promise that resolves to transaction hash
   */
  async submitTransactionHelper(account: AptosAccount, payload: TxnBuilderTypes.TransactionPayload) {
    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      this.aptosClient.getAccount(account.address()),
      this.aptosClient.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account.address()),
      BigInt(sequnceNumber),
      payload,
      500000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 20),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const transactionRes = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);

    return transactionRes.hash;
  }

  /**
   * Creates a new NFT collection within the specified account
   * @param account AptosAccount where collection will be created
   * @param name Collection name
   * @param description Collection description
   * @param uri URL to additional info about collection
   * @param maxAmount Maximum number of `token_data` allowed within this collection
   * @returns A hash of transaction
   */
  async createCollection(
    account: AptosAccount,
    name: string,
    description: string,
    uri: string,
    maxAmount: BCS.AnyNumber = MAX_U64_BIG_INT,
  ): Promise<string> {
    const payload = this.transactionBuilder.buildTransactionPayload(
      "0x3::token::create_collection_script",
      [],
      [name, description, uri, maxAmount, [false, false, false]],
    );

    return this.submitTransactionHelper(account, payload);
  }

  /**
   * Creates a new NFT within the specified account
   * @param account AptosAccount where token will be created
   * @param collectionName Name of collection, that token belongs to
   * @param name Token name
   * @param description Token description
   * @param supply Token supply
   * @param uri URL to additional info about token
   * @param max The maxium of tokens can be minted from this token
   * @param royalty_payee_address the address to receive the royalty, the address can be a shared account address.
   * @param royalty_points_denominator the denominator for calculating royalty
   * @param royalty_points_numerator the numerator for calculating royalty
   * @param property_keys the property keys for storing on-chain properties
   * @param property_values the property values to be stored on-chain
   * @param property_types the type of property values
   * @returns A hash of transaction
   */
  async createToken(
    account: AptosAccount,
    collectionName: string,
    name: string,
    description: string,
    supply: number,
    uri: string,
    max: BCS.AnyNumber = MAX_U64_BIG_INT,
    royalty_payee_address: MaybeHexString = account.address(),
    royalty_points_denominator: number = 0,
    royalty_points_numerator: number = 0,
    property_keys: Array<string> = [],
    property_values: Array<string> = [],
    property_types: Array<string> = [],
  ): Promise<Gen.HexEncodedBytes> {
    const payload = this.transactionBuilder.buildTransactionPayload(
      "0x3::token::create_token_script",
      [],
      [
        collectionName,
        name,
        description,
        supply,
        max,
        uri,
        royalty_payee_address,
        royalty_points_denominator,
        royalty_points_numerator,
        [false, false, false, false, false],
        property_keys,
        property_values,
        property_types,
      ],
    );

    return this.submitTransactionHelper(account, payload);
  }

  /**
   * Transfers specified amount of tokens from account to receiver
   * @param account AptosAccount where token from which tokens will be transfered
   * @param receiver  Hex-encoded 32 byte Aptos account address to which tokens will be transfered
   * @param creator Hex-encoded 32 byte Aptos account address to which created tokens
   * @param collectionName Name of collection where token is stored
   * @param name Token name
   * @param amount Amount of tokens which will be transfered
   * @param property_version the version of token PropertyMap with a default value 0.
   * @returns A hash of transaction
   */
  async offerToken(
    account: AptosAccount,
    receiver: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
    amount: number,
    property_version: number = 0,
  ): Promise<string> {
    const payload = this.transactionBuilder.buildTransactionPayload(
      "0x3::token_transfers::offer_script",
      [],
      [receiver, creator, collectionName, name, property_version, amount],
    );

    return this.submitTransactionHelper(account, payload);
  }

  /**
   * Claims a token on specified account
   * @param account AptosAccount which will claim token
   * @param sender Hex-encoded 32 byte Aptos account address which holds a token
   * @param creator Hex-encoded 32 byte Aptos account address which created a token
   * @param collectionName Name of collection where token is stored
   * @param name Token name
   * @param property_version the version of token PropertyMap with a default value 0.
   * @returns A hash of transaction
   */
  async claimToken(
    account: AptosAccount,
    sender: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
    property_version: number = 0,
  ): Promise<string> {
    const payload = this.transactionBuilder.buildTransactionPayload(
      "0x3::token_transfers::claim_script",
      [],
      [sender, creator, collectionName, name, property_version],
    );

    return this.submitTransactionHelper(account, payload);
  }

  /**
   * Removes a token from pending claims list
   * @param account AptosAccount which will remove token from pending list
   * @param receiver Hex-encoded 32 byte Aptos account address which had to claim token
   * @param creator Hex-encoded 32 byte Aptos account address which created a token
   * @param collectionName Name of collection where token is strored
   * @param name Token name
   * @param property_version the version of token PropertyMap with a default value 0.
   * @returns A hash of transaction
   */
  async cancelTokenOffer(
    account: AptosAccount,
    receiver: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
    property_version: number = 0,
  ): Promise<string> {
    const payload = this.transactionBuilder.buildTransactionPayload(
      "0x3::token_transfers::cancel_offer_script",
      [],
      [receiver, creator, collectionName, name, property_version],
    );

    return this.submitTransactionHelper(account, payload);
  }

  /**
   * Queries collection data
   * @param creator Hex-encoded 32 byte Aptos account address which created a collection
   * @param collectionName Collection name
   * @returns Collection data in below format
   * ```
   *  Collection {
   *    // Describes the collection
   *    description: string,
   *    // Unique name within this creators account for this collection
   *    name: string,
   *    // URL for additional information/media
   *    uri: string,
   *    // Total number of distinct Tokens tracked by the collection
   *    count: number,
   *    // Optional maximum number of tokens allowed within this collections
   *    maximum: number
   *  }
   * ```
   */
  async getCollectionData(creator: MaybeHexString, collectionName: string): Promise<any> {
    const resources = await this.aptosClient.getAccountResources(creator);
    const accountResource: { type: Gen.MoveStructTag; data: any } = resources.find(
      (r) => r.type === "0x3::token::Collections",
    )!;
    const { handle }: { handle: string } = accountResource.data.collection_data;
    const getCollectionTableItemRequest: Gen.TableItemRequest = {
      key_type: "0x1::string::String",
      value_type: "0x3::token::CollectionData",
      key: collectionName,
    };

    const collectionTable = await this.aptosClient.getTableItem(handle, getCollectionTableItemRequest);
    return collectionTable;
  }

  /**
   * Queries token data from collection
   * @param creator Hex-encoded 32 byte Aptos account address which created a token
   * @param collectionName Name of collection, which holds a token
   * @param tokenName Token name
   * @returns Token data in below format
   * ```
   * TokenData {
   *     // Unique name within this creators account for this Token's collection
   *     collection: string;
   *     // Describes this Token
   *     description: string;
   *     // The name of this Token
   *     name: string;
   *     // Optional maximum number of this type of Token.
   *     maximum: number;
   *     // Total number of this type of Token
   *     supply: number;
   *     /// URL for additional information / media
   *     uri: string;
   *   }
   * ```
   */
  async getTokenData(
    creator: MaybeHexString,
    collectionName: string,
    tokenName: string,
  ): Promise<TokenTypes.TokenData> {
    const collection: { type: Gen.MoveStructTag; data: any } = await this.aptosClient.getAccountResource(
      creator,
      "0x3::token::Collections",
    );
    const { handle } = collection.data.token_data;
    const tokenDataId = {
      creator,
      collection: collectionName,
      name: tokenName,
    };

    const getTokenTableItemRequest: Gen.TableItemRequest = {
      key_type: "0x3::token::TokenDataId",
      value_type: "0x3::token::TokenData",
      key: tokenDataId,
    };

    // We know the response will be a struct containing TokenData, hence the
    // implicit cast.
    return this.aptosClient.getTableItem(handle, getTokenTableItemRequest);
  }

  /**
   * Queries token balance for the token creator
   */
  async getToken(
    creator: MaybeHexString,
    collectionName: string,
    tokenName: string,
    property_version: string = "0",
  ): Promise<TokenTypes.Token> {
    const tokenDataId: TokenTypes.TokenDataId = {
      creator: creator instanceof HexString ? creator.hex() : creator,
      collection: collectionName,
      name: tokenName,
    };
    return this.getTokenForAccount(creator, {
      token_data_id: tokenDataId,
      property_version,
    });
  }

  /**
   * TODO: What does this mean? Is it more like getTokenBalanceInAccount?
   * Queries token balance for a token account
   * @param account Hex-encoded 32 byte Aptos account address which created a token
   * @param tokenId token id
   *
   * TODO: Update this:
   * @example
   * ```
   * {
   *   creator: '0x1',
   *   collection: 'Some collection',
   *   name: 'Awesome token'
   * }
   * ```
   * @returns Token object in below format
   * ```
   * Token {
   *   id: TokenId;
   *   value: number;
   * }
   * ```
   */
  async getTokenForAccount(account: MaybeHexString, tokenId: TokenTypes.TokenId): Promise<TokenTypes.Token> {
    const tokenStore: { type: Gen.MoveStructTag; data: any } = await this.aptosClient.getAccountResource(
      account,
      "0x3::token::TokenStore",
    );
    const { handle } = tokenStore.data.tokens;

    const getTokenTableItemRequest: Gen.TableItemRequest = {
      key_type: "0x3::token::TokenId",
      value_type: "0x3::token::Token",
      key: tokenId,
    };

    try {
      return await this.aptosClient.getTableItem(handle, getTokenTableItemRequest);
    } catch (error) {
      if (error.status === 404) {
        return {
          id: tokenId,
          amount: "0",
        };
      }
      return error;
    }
  }
}
