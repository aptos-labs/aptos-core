import { DEFAULT_NETWORK } from "../utils";

export class AptosConfig {
  readonly network: string;

  constructor(config?: AptosConfig) {
    this.network = config?.network ?? DEFAULT_NETWORK;
  }
}
