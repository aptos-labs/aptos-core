import { AptosConfig } from "./aptos_config";
import { Gen } from "../types";
import { Memoize, parseApiError } from "../utils";
import { getBlockByHeight, getBlockByVersion, getChainId, getLedgerInfo, view } from "../internal/general";

export class General {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  /**
   * Queries the latest ledger information
   * @returns Latest ledger information
   * @example Example of returned data
   * ```
   * {
   *   chain_id: 15,
   *   epoch: 6,
   *   ledgerVersion: "2235883",
   *   ledger_timestamp:"1654580922321826"
   * }
   * ```
   */
  @parseApiError
  async getLedgerInfo(): Promise<Gen.IndexResponse> {
    const info = await getLedgerInfo(this.config);
    return info;
  }

  /**
   * @returns Current chain id
   */
  @Memoize()
  async getChainId(): Promise<number> {
    const chainId = await getChainId(this.config);
    return chainId;
  }

  /**
   * Call for a move view function
   *
   * @param payload Transaction payload
   * @param version (optional) Ledger version to lookup block information for
   *
   * @returns MoveValue[]
   */
  @parseApiError
  async view(payload: Gen.ViewRequest, ledger_version?: string): Promise<Gen.MoveValue[]> {
    const data = await view(this.config, payload, ledger_version);
    return data;
  }

  /**
   * Get block by height
   *
   * @param blockHeight Block height to lookup.  Starts at 0
   * @param withTransactions If set to true, include all transactions in the block
   *
   * @returns Block
   */
  @parseApiError
  async getBlockByHeight(blockHeight: number, withTransactions?: boolean): Promise<Gen.Block> {
    const block = await getBlockByHeight(this.config, blockHeight, withTransactions);
    return block;
  }

  /**
   * Get block by block transaction version
   *
   * @param version Ledger version to lookup block information for
   * @param withTransactions If set to true, include all transactions in the block
   *
   * @returns Block
   */
  @parseApiError
  async getBlockByVersion(version: number, withTransactions?: boolean): Promise<Gen.Block> {
    const block = await getBlockByVersion(this.config, version, withTransactions);
    return block;
  }
}
