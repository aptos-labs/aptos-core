import { ClientConfig } from "../client/types";

export class AptosConfig {
  readonly clientConfig?: ClientConfig;

  constructor(config?: AptosConfig) {
    this.clientConfig = config?.clientConfig ?? {};
  }
}
