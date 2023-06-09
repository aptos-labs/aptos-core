import { AptosConfig } from "./aptos_config";

export class Aptos {
  readonly config: AptosConfig;

  constructor(settings?: AptosConfig) {
    // can read from existing conf file or use the input settings param
    this.config = new AptosConfig(settings);
  }
}
