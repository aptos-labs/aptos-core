import { getBlockByHeight, getBlockByVersion, getLedgerInfo, getTableItem, view } from "../internal/general";
import { Block, LedgerInfo, LedgerVersion, MoveValue, TableItemRequest, ViewRequest } from "../types";
import { AptosConfig } from "./aptos_config";

/**
 * A class to query all `General` Aptos related queries
 */
export class General {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * Queries for the Aptos ledger info
   *
   * @returns Aptos Ledger Info
   *
   * @example An example of the returned data
   * ```
   * {
   * "chain_id": 4,
   * "epoch": "8",
   * "ledger_version": "714",
   * "oldest_ledger_version": "0",
   * "ledger_timestamp": "1694695496521775",
   * "node_role": "validator",
   * "oldest_block_height": "0",
   * "block_height": "359",
   * "git_hash": "c82193f36f4e185fed9f68c4ad21f6c6dd390c6e"
   * }
   * ```
   */
  async getLedgerInfo(): Promise<LedgerInfo> {
    const data = await getLedgerInfo({ aptosConfig: this.config });
    return data;
  }

  /**
   * Queries for the chain id
   *
   * @returns The chain id
   */
  async getChainId(): Promise<number> {
    const result = await this.getLedgerInfo();
    return result.chain_id;
  }

  /**
   * Queries for block by transaction version
   *
   * @param version Ledger version to lookup block information for
   * @param options.withTransactions If set to true, include all transactions in the block
   *
   * @returns Block
   */
  async getBlockByVersion(args: { blockVersion: number; options?: { withTransactions?: boolean } }): Promise<Block> {
    const block = await getBlockByVersion({ aptosConfig: this.config, ...args });
    return block;
  }

  /**
   * Get block by block height
   *
   * @param blockHeight Block height to lookup.  Starts at 0
   * @param options.withTransactions If set to true, include all transactions in the block
   *
   * @returns Block
   */
  async getBlockByHeight(args: { blockHeight: number; options?: { withTransactions?: boolean } }): Promise<Block> {
    const block = await getBlockByHeight({ aptosConfig: this.config, ...args });
    return block;
  }

  /**
   * Queries for a table item for a table identified by the handle and the key for the item.
   * Key and value types need to be passed in to help with key serialization and value deserialization.
   * @param handle A pointer to where that table is stored
   * @param data Object that describes table item
   *
   * @example https://fullnode.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::coin::CoinInfo%3C0x1::aptos_coin::AptosCoin%3E
   * {
   *  data.key_type = "address" // Move type of table key
   *  data.value_type = "u128" // Move type of table value
   *  data.key = "0x619dc29a0aac8fa146714058e8dd6d2d0f3bdf5f6331907bf91f3acd81e6935" // Value of table key
   * }
   *
   * @returns Table item value rendered in JSON
   */
  async getTableItem(args: { handle: string; data: TableItemRequest; options?: LedgerVersion }): Promise<any> {
    const item = await getTableItem({ aptosConfig: this.config, ...args });
    return item;
  }

  /**
   * Queries for a Move view function
   * @param payload ViewRequest payload
   * @example
   * `
   * const payload: ViewRequest = {
   *  function: "0x1::coin::balance",
   *  type_arguments: ["0x1::aptos_coin::AptosCoin"],
   *  arguments: [accountAddress],
   * };
   * `
   *
   * @returns a Move value
   */
  async view(args: { payload: ViewRequest; options?: LedgerVersion }): Promise<MoveValue> {
    const data = await view({ aptosConfig: this.config, ...args });
    return data[0];
  }
}
