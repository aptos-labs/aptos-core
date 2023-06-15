import { ClientConfig } from "../client/types";
import { DEFAULT_NETWORK, DEFAULT_FAUCET } from "../utils";

export class AptosConfig {
  readonly network: string;

  readonly faucet: string;

  readonly clientConfig?: ClientConfig;

  constructor(config?: AptosConfig) {
    this.network = config?.network ?? DEFAULT_NETWORK;
    this.faucet = config?.faucet ?? DEFAULT_FAUCET;
    this.clientConfig = config?.clientConfig ?? {};
  }
}
