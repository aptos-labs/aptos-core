import { AptosConfig } from "./aptos_config";
import {
  AccountData,
  LedgerVersion,
  MoveModuleBytecode,
  MoveResource,
  MoveResourceType,
  PaginationArgs,
  TransactionResponse,
  HexInput,
  IndexerPaginationArgs,
  GetAccountTokensCountQueryResponse,
  TokenStandard,
  OrderBy,
  GetAccountOwnedTokensQueryResponse,
  GetAccountCollectionsWithOwnedTokenResponse,
  GetAccountTransactionsCountResponse,
  GetAccountCoinsDataResponse,
  GetAccountCoinsCountResponse,
  GetAccountOwnedObjectsResponse,
  GetAccountOwnedTokensFromCollectionResponse,
} from "../types";
import {
  getAccountCoinsCount,
  getAccountCoinsData,
  getAccountCollectionsWithOwnedTokens,
  getAccountOwnedObjects,
  getAccountOwnedTokens,
  getAccountOwnedTokensFromCollectionAddress,
  getAccountTokensCount,
  getAccountTransactionsCount,
  getInfo,
  getModule,
  getModules,
  getResource,
  getResources,
  getTransactions,
  lookupOriginalAccountAddress,
} from "../internal/account";
import { Hex } from "../core/hex";

/**
 * A class to query all `Account` related queries on Aptos.
 */
export class Account {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * Queries for an Aptos account given an account address
   *
   * @param accountAddress Aptos account address
   *
   * @returns The account data
   *
   * @example An example of the returned account
   * ```
   * {
   *    sequence_number: "1",
   *    authentication_key: "0x5307b5f4bc67829097a8ba9b43dba3b88261eeccd1f709d9bde240fc100fbb69"
   * }
   * ```
   */
  async getAccountInfo(args: { accountAddress: HexInput }): Promise<AccountData> {
    const data = await getInfo({ aptosConfig: this.config, ...args });
    return data;
  }

  /**
   * Queries for an acount modules given an account address
   *
   * Note: In order to get all account modules, this function may call the API
   * multiple times as it auto paginates.
   *
   * @param accountAddress Aptos account address
   * @returns Account modules
   */

  async getAccountModules(args: {
    accountAddress: HexInput;
    options?: PaginationArgs & LedgerVersion;
  }): Promise<MoveModuleBytecode[]> {
    const modules = await getModules({ aptosConfig: this.config, ...args });
    return modules;
  }

  /**
   * Queries for an account module given account address and module name
   *
   * @param accountAddress Aptos account address
   * @param moduleName The name of the module
   *
   * @returns Account module
   *
   * @example An example of an account module
   * ```
   * {
   *    bytecode: "0xa11ceb0b0600000006010002030206050807070f0d081c200",
   *    abi: { address: "0x1" }
   * }
   * ```
   */
  async getAccountModule(args: {
    accountAddress: HexInput;
    moduleName: string;
    options?: LedgerVersion;
  }): Promise<MoveModuleBytecode> {
    const module = await getModule({ aptosConfig: this.config, ...args });
    return module;
  }

  /**
   * Queries account transactions given an account address
   *
   * Note: In order to get all account transactions, this function may call the API
   * multiple times as it auto paginates.
   *
   * @param accountAddress Aptos account address
   *
   * @returns The account transactions
   */
  async getAccountTransactions(args: {
    accountAddress: HexInput;
    options?: PaginationArgs;
  }): Promise<TransactionResponse[]> {
    const transactions = await getTransactions({ aptosConfig: this.config, ...args });
    return transactions;
  }

  /**
   * Queries account resources given an account address
   *
   * Note: In order to get all account resources, this function may call the API
   * multiple times as it auto paginates.
   *
   * @param accountAddress Aptos account address
   * @returns Account resources
   */
  async getAccountResources(args: {
    accountAddress: HexInput;
    options?: PaginationArgs & LedgerVersion;
  }): Promise<MoveResource[]> {
    const resources = await getResources({ aptosConfig: this.config, ...args });
    return resources;
  }

  /**
   * Queries account resource given account address and resource type
   *
   * @param accountAddress Aptos account address
   * @param resourceType String representation of an on-chain Move struct type, i.e "0x1::aptos_coin::AptosCoin"
   *
   * @returns Account resource
   *
   * @example An example of an account resource
   * ```
   * {
   *    type: "0x1::aptos_coin::AptosCoin",
   *    data: { value: 6 }
   * }
   * ```
   */
  async getAccountResource(args: {
    accountAddress: HexInput;
    resourceType: MoveResourceType;
    options?: LedgerVersion;
  }): Promise<MoveResource> {
    const resource = await getResource({ aptosConfig: this.config, ...args });
    return resource;
  }

  /**
   * Lookup the original address by the current derived address or authentication key
   *
   * @param args.addressOrAuthKey The derived address or authentication key
   * @returns Promise<Hex> The original address
   */
  async lookupOriginalAccountAddress(args: { addressOrAuthKey: HexInput; options?: LedgerVersion }): Promise<Hex> {
    const address = await lookupOriginalAccountAddress({ aptosConfig: this.config, ...args });
    return address;
  }

  /**
   * Queries the count of tokens owned by an account
   *
   * @param accountAddress The account address
   * @returns An object { count : number }
   */
  async getAccountTokensCount(args: { accountAddress: HexInput }): Promise<GetAccountTokensCountQueryResponse> {
    const count = await getAccountTokensCount({ aptosConfig: this.config, ...args });
    return count;
  }

  /**
   * Queries the account's current owned tokens.
   *
   * This query returns all tokens (v1 and v2 standards) an account owns, including NFTs, fungible, soulbound, etc.
   * If you want to get only the token from a specific standrd, you can pass an optional tokenStandard param
   *
   * @param accountAddress The account address we want to get the tokens for
   * @returns Tokens array with the token data
   */
  async getAccountOwnedTokens(args: {
    accountAddress: HexInput;
    options?: {
      tokenStandard?: TokenStandard;
      pagination?: IndexerPaginationArgs;
      orderBy?: OrderBy<GetAccountOwnedTokensQueryResponse[0]>;
    };
  }): Promise<GetAccountOwnedTokensQueryResponse> {
    const tokens = await getAccountOwnedTokens({ aptosConfig: this.config, ...args });
    return tokens;
  }

  /**
   * Queries all tokens of a specific collection that an account owns by the collection address
   *
   * This query returns all tokens (v1 and v2 standards) an account owns, including NFTs, fungible, soulbound, etc.
   * If you want to get only the token from a specific standrd, you can pass an optional tokenStandard param
   *
   * @param ownerAddress The account address we want to get the tokens for
   * @param collectionAddress The address of the collection being queried
   * @returns Tokens array with the token data
   */
  async getAccountOwnedTokensFromCollectionAddress(args: {
    ownerAddress: HexInput;
    collectionAddress: HexInput;
    options?: {
      tokenStandard?: TokenStandard;
      pagination?: IndexerPaginationArgs;
      orderBy?: OrderBy<GetAccountOwnedTokensFromCollectionResponse[0]>;
    };
  }): Promise<GetAccountOwnedTokensFromCollectionResponse> {
    const tokens = await getAccountOwnedTokensFromCollectionAddress({ aptosConfig: this.config, ...args });
    return tokens;
  }

  /**
   * Queries for all collections that an account has tokens for.
   *
   * This query returns all tokens (v1 and v2 standards) an account owns, including NFTs, fungible, soulbound, etc.
   * If you want to get only the token from a specific standrd, you can pass an optional tokenStandard param
   *
   * @param accountAddress The account address we want to get the collections for
   * @returns Collections array with the collections data
   */
  async getAccountCollectionsWithOwnedTokens(args: {
    accountAddress: HexInput;
    options?: {
      tokenStandard?: TokenStandard;
      pagination?: IndexerPaginationArgs;
      orderBy?: OrderBy<GetAccountCollectionsWithOwnedTokenResponse[0]>;
    };
  }): Promise<GetAccountCollectionsWithOwnedTokenResponse> {
    const collections = await getAccountCollectionsWithOwnedTokens({ aptosConfig: this.config, ...args });
    return collections;
  }

  /**
   * Queries the count of transactions submitted by an account
   *
   * @param accountAddress The account address we want to get the total count for
   * @returns An object { count : number }
   */
  async getAccountTransactionsCount(args: { accountAddress: HexInput }): Promise<GetAccountTransactionsCountResponse> {
    const count = getAccountTransactionsCount({ aptosConfig: this.config, ...args });
    return count;
  }

  /**
   * Queries an account's coins data
   *
   * @param accountAddress The account address we want to get the coins data for
   * @returns Array with the coins data
   */
  async getAccountCoinsData(args: {
    accountAddress: HexInput;
    options?: {
      pagination?: IndexerPaginationArgs;
      orderBy?: OrderBy<GetAccountCoinsDataResponse[0]>;
    };
  }): Promise<GetAccountCoinsDataResponse> {
    const data = await getAccountCoinsData({ aptosConfig: this.config, ...args });
    return data;
  }

  /**
   * Queries the count of an account's coins
   *
   * @param accountAddress The account address we want to get the total count for
   * @returns An object { count : number }
   */
  async getAccountCoinsCount(args: { accountAddress: HexInput }): Promise<GetAccountCoinsCountResponse> {
    const count = getAccountCoinsCount({ aptosConfig: this.config, ...args });
    return count;
  }

  /**
   * Queries an account's owned objects
   *
   * @param ownerAddress The account address we want to get the objects for
   * @returns Objects array with the object data
   */
  async getAccountOwnedObjects(args: {
    ownerAddress: HexInput;
    options?: {
      pagination?: IndexerPaginationArgs;
      orderBy?: OrderBy<GetAccountOwnedObjectsResponse[0]>;
    };
  }): Promise<GetAccountOwnedObjectsResponse> {
    const objects = getAccountOwnedObjects({ aptosConfig: this.config, ...args });
    return objects;
  }
}
