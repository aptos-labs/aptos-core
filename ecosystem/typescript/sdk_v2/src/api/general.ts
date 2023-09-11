import { getLedgerInfo } from "../internal/general";
import { LedgerInfo } from "../types";
import { AptosConfig } from "./aptos_config";

export class General {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  async getLedgerInfo(): Promise<LedgerInfo> {
    const data = await getLedgerInfo({ aptosConfig: this.config });
    return data;
  }

  async getChainId(): Promise<number> {
    const result = await this.getLedgerInfo();
    return result.chain_id;
  }
}
