import { AptosConfig } from "./aptos_config";

export class Aptos {
  readonly config: AptosConfig;

  /**
   * This class is the main entry point into Aptos's
   * APIs and separates functionality into different namespaces.
   *
   * To use the SDK, create a new Aptos instance to get access
   * to all the sdk functionality.
   * @example
   * ```
   * {
   * const config: AptosConfig = {network:Network.TESTNET}
   * const aptos = new Aptos(config);
   * await aptos.account.getData("0x1")
   * }
   * ```
   *
   */
  constructor(settings?: AptosConfig) {
    this.config = new AptosConfig(settings);
  }
}
