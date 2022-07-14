import { AptosAccount } from './aptos_account';
import { AptosClient } from './aptos_client';
import { Types } from './types';
import { HexString, MaybeHexString } from './hex_string';
import { TxnBuilderTypes, BCS } from './transaction_builder';

/**
 * Class for creating, minting and managing minting NFT collections and tokens
 */
export class TokenClient {
  aptosClient: AptosClient;

  /**
   * Creates new TokenClient instance
   * @param aptosClient AptosClient instance
   */
  constructor(aptosClient: AptosClient) {
    this.aptosClient = aptosClient;
  }

  private async submitTransactionHelper(
    account: AptosAccount,
    rawTxn: TxnBuilderTypes.RawTransaction,
  ): Promise<string> {
    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const transactionRes = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);

    await this.aptosClient.waitForTransaction(transactionRes.hash);
    return transactionRes.hash;
  }

  /**
   * Creates a new NFT collection within the specified account
   * @param account AptosAccount where collection will be created
   * @param name Collection name
   * @param description Collection description
   * @param uri URL to additional info about collection
   * @returns A hash of transaction
   */
  async createCollection(
    account: AptosAccount,
    name: string,
    description: string,
    uri: string,
  ): Promise<Types.HexEncodedBytes> {
    const payload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        '0x1::Token',
        'create_unlimited_collection_script',
        [],
        [BCS.bcsSerializeStr(name), BCS.bcsSerializeStr(description), BCS.bcsSerializeStr(uri)],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      this.aptosClient.getAccount(account.address()),
      this.aptosClient.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account.address()),
      BigInt(sequnceNumber),
      payload,
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    return this.submitTransactionHelper(account, rawTxn);
  }

  /**
   * Creates a new NFT within the specified account
   * @param account AptosAccount where token will be created
   * @param collectionName Name of collection, that token belongs to
   * @param name Token name
   * @param description Token description
   * @param supply Token supply
   * @param uri URL to additional info about token
   * @param royalty_points_per_million the royal points to be provided to creator
   * @returns A hash of transaction
   */
  async createToken(
    account: AptosAccount,
    collectionName: string,
    name: string,
    description: string,
    supply: number,
    uri: string,
    royalty_points_per_million: number,
  ): Promise<Types.HexEncodedBytes> {
    const payload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        '0x1::Token',
        'create_unlimited_token_script',
        [],
        [
          BCS.bcsSerializeStr(collectionName),
          BCS.bcsSerializeStr(name),
          BCS.bcsSerializeStr(description),
          BCS.bcsSerializeBool(true),
          BCS.bcsSerializeUint64(supply),
          BCS.bcsSerializeStr(uri),
          BCS.bcsSerializeUint64(royalty_points_per_million),
        ],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      this.aptosClient.getAccount(account.address()),
      this.aptosClient.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account.address()),
      BigInt(sequnceNumber),
      payload,
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const transactionRes = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);

    await this.aptosClient.waitForTransaction(transactionRes.hash);
    return transactionRes.hash;
  }

  /**
   * Transfers specified amount of tokens from account to receiver
   * @param account AptosAccount where token from which tokens will be transfered
   * @param receiver  Hex-encoded 16 bytes Aptos account address to which tokens will be transfered
   * @param creator Hex-encoded 16 bytes Aptos account address to which created tokens
   * @param collectionName Name of collection where token is stored
   * @param name Token name
   * @param amount Amount of tokens which will be transfered
   * @returns A hash of transaction
   */
  async offerToken(
    account: AptosAccount,
    receiver: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
    amount: number,
  ): Promise<Types.HexEncodedBytes> {
    const payload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        '0x1::TokenTransfers',
        'offer_script',
        [],
        [
          BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(receiver)),
          BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator)),
          BCS.bcsSerializeStr(collectionName),
          BCS.bcsSerializeStr(name),
          BCS.bcsSerializeUint64(amount),
        ],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      this.aptosClient.getAccount(account.address()),
      this.aptosClient.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account.address()),
      BigInt(sequnceNumber),
      payload,
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    return this.submitTransactionHelper(account, rawTxn);
  }

  /**
   * Claims a token on specified account
   * @param account AptosAccount which will claim token
   * @param sender Hex-encoded 16 bytes Aptos account address which holds a token
   * @param creator Hex-encoded 16 bytes Aptos account address which created a token
   * @param collectionName Name of collection where token is stored
   * @param name Token name
   * @returns A hash of transaction
   */
  async claimToken(
    account: AptosAccount,
    sender: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
  ): Promise<Types.HexEncodedBytes> {
    const payload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        '0x1::TokenTransfers',
        'claim_script',
        [],
        [
          BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(sender)),
          BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator)),
          BCS.bcsSerializeStr(collectionName),
          BCS.bcsSerializeStr(name),
        ],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      this.aptosClient.getAccount(account.address()),
      this.aptosClient.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account.address()),
      BigInt(sequnceNumber),
      payload,
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    return this.submitTransactionHelper(account, rawTxn);
  }

  /**
   * Removes a token from pending claims list
   * @param account AptosAccount which will remove token from pending list
   * @param receiver Hex-encoded 16 bytes Aptos account address which had to claim token
   * @param creator Hex-encoded 16 bytes Aptos account address which created a token
   * @param collectionName Name of collection where token is strored
   * @param name Token name
   * @returns A hash of transaction
   */
  async cancelTokenOffer(
    account: AptosAccount,
    receiver: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
  ): Promise<Types.HexEncodedBytes> {
    const payload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        '0x1::TokenTransfers',
        'cancel_offer_script',
        [],
        [
          BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(receiver)),
          BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(creator)),
          BCS.bcsSerializeStr(collectionName),
          BCS.bcsSerializeStr(name),
        ],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      this.aptosClient.getAccount(account.address()),
      this.aptosClient.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account.address()),
      BigInt(sequnceNumber),
      payload,
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    return this.submitTransactionHelper(account, rawTxn);
  }

  /**
   * Queries collection data
   * @param creator Hex-encoded 16 bytes Aptos account address which created a collection
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
    const accountResource: { type: string; data: any } = resources.find((r) => r.type === '0x1::Token::Collections');
    const { handle }: { handle: string } = accountResource.data.collections;
    const getCollectionTableItemRequest: Types.TableItemRequest = {
      key_type: '0x1::ASCII::String',
      value_type: '0x1::Token::Collection',
      key: collectionName,
    };
    // eslint-disable-next-line no-unused-vars
    const collectionTable = await this.aptosClient.getTableItem(handle, getCollectionTableItemRequest);
    return collectionTable;
  }

  /**
   * Queries token data from collection
   * @param creator Hex-encoded 16 bytes Aptos account address which created a token
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
  async getTokenData(creator: MaybeHexString, collectionName: string, tokenName: string): Promise<Types.TokenData> {
    const collection: { type: string; data: any } = await this.aptosClient.getAccountResource(
      creator,
      '0x1::Token::Collections',
    );
    const { handle } = collection.data.token_data;
    const tokenId = {
      creator,
      collection: collectionName,
      name: tokenName,
    };

    const getTokenTableItemRequest: Types.TableItemRequest = {
      key_type: '0x1::Token::TokenId',
      value_type: '0x1::Token::TokenData',
      key: tokenId,
    };

    const tableItem = await this.aptosClient.getTableItem(handle, getTokenTableItemRequest);
    return tableItem.data;
  }

  /**
   * Queries token balance for the token creator
   * @deprecated Use getTokenBalanceForAccount instead
   */
  async getTokenBalance(creator: MaybeHexString, collectionName: string, tokenName: string): Promise<Types.Token> {
    return this.getTokenBalanceForAccount(creator, {
      creator: creator instanceof HexString ? creator.hex() : creator,
      collection: collectionName,
      name: tokenName,
    });
  }

  /**
   * Queries token balance for a token account
   * @param account Hex-encoded 16 bytes Aptos account address which created a token
   * @param tokenId token id
   *
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
  async getTokenBalanceForAccount(account: MaybeHexString, tokenId: Types.TokenId): Promise<Types.Token> {
    const tokenStore: { type: string; data: any } = await this.aptosClient.getAccountResource(
      account,
      '0x1::Token::TokenStore',
    );
    const { handle } = tokenStore.data.tokens;

    const getTokenTableItemRequest: Types.TableItemRequest = {
      key_type: '0x1::Token::TokenId',
      value_type: '0x1::Token::Token',
      key: tokenId,
    };

    const tableItem = await this.aptosClient.getTableItem(handle, getTokenTableItemRequest);
    return tableItem.data;
  }
}
