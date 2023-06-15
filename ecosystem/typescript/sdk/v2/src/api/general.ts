import { AptosConfig } from "./aptos_config";
import { get } from "../client";

export class General {
  readonly config: AptosConfig;

  constructor(config: AptosConfig) {
    this.config = config;
  }

  async getLedgerInfo(): Promise<any> {
    return await get(this.config, `/`, null, "getLedgerInfo");
  }

  async getChainId(): Promise<any> {
    const data = await this.getLedgerInfo();
    return data.chain_id;
  }
}
