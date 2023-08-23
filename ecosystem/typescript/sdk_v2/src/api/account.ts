import { AptosConfig } from "./aptos_config";
import {
  AccountData,
  LedgerVersion,
  MoveModuleBytecode,
  MoveResource,
  MoveResourceType,
  PaginationArgs,
  Transaction,
  HexInput,
} from "../types";
import { getInfo, getModule, getModules, getResource, getResources, getTransactions } from "../internal/account";

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
  async getInfo(args: { accountAddress: HexInput }): Promise<AccountData> {
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

  async getModules(args: {
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
  async getModule(args: {
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
  async getTransactions(args: { accountAddress: HexInput; options?: PaginationArgs }): Promise<Transaction[]> {
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
  async getResources(args: {
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
  async getResource(args: {
    accountAddress: HexInput;
    resourceType: MoveResourceType;
    options?: LedgerVersion;
  }): Promise<MoveResource> {
    const resource = await getResource({ aptosConfig: this.config, ...args });
    return resource;
  }
}
