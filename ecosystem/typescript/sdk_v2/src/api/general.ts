import { AptosConfig } from "./aptos_config";
import { Gen } from "../types";
import { Memoize, parseApiError } from "../utils";
import { getBlockByHeight, getBlockByVersion, getChainId, getLedgerInfo, view } from "../internal/general";
import { ABIRoot, ExtractReturnType, ViewFunctionName, ViewRequestPayload } from "../type_utils";

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
  * @param ledger_version (optional) Ledger version to lookup block information for
  *
  * @returns MoveValue[]
  */
  async view(payload: Gen.ViewRequest, ledger_version?: string): Promise<Gen.MoveValue[]>

  /**
   * Call for a move view function with type safety
   *
   * @template TABI - The ABI JSON you want to call
   * @template TFuncName - The function name in the ABI you want to call
   * 
   * @param payload Transaction payload
   * @param ledger_version (optional) Ledger version to lookup block information for
   * @
   * @returns A readonly array of the return types of the function you called
   * @example
   * const [balance] = await client.view<COIN_ABI, "balance">({
   *     function: "0x1::coin::balance",
   *     arguments: ["0x1"],
   *     type_arguments: ["0x1"],
   * });
   */
  async view<
    TABI extends ABIRoot,
    TFuncName extends ViewFunctionName<TABI>>(
      payload: ViewRequestPayload<TABI>,
      ledger_version?: string): Promise<ExtractReturnType<TABI, TFuncName>>

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
