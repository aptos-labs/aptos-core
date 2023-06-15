import { AptosConfig } from "./aptos_config";
import { get } from "../client";
import { MaybeHexString, PaginationArgs } from "../types";
import { AccountData, MoveModuleBytecode } from "../types/generated";

export class Account {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }
  // TODO use HexString type
  async getData(accountAddress: MaybeHexString, ledgerVersion?: bigint): Promise<AccountData> {
    return await get(this.config, `accounts/${accountAddress}`, ledgerVersion, "getData");
  }

  async getModules(accountAddress: MaybeHexString, ledgerVersion?: bigint): Promise<MoveModuleBytecode[]> {
    return await get(this.config, `/accounts/${accountAddress}/modules`, ledgerVersion, "getModules");
  }

  async getCoinsData(accountAddress: MaybeHexString, query?: PaginationArgs) {}
}
