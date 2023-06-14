import { AptosConfig } from "../aptos_config";
import { get, post } from "../client";
import { AccountData, PaginationArgs } from "../types";

export class Account {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }
  // TODO use HexString type
  async get(accountAddress: string, ledgerVersion?: bigint): Promise<AccountData> {
    return await get(this.config, `accounts/${accountAddress}`, ledgerVersion, "getAccount");
  }

  async getCoinsData(accountAddress: string, query?: PaginationArgs) {}

  // TODO move to Transaction class
  async submitTransaction(signedTxn: Uint8Array) {
    return await post(this.config, `/transactions`, signedTxn, "submitTransaction", {
      headers: { "Content-Type": "application/x.aptos.signed_transaction+bcs" },
    });
  }
}
