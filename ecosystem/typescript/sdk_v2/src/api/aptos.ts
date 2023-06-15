import { Account } from "./account";
import { AptosConfig } from "./aptos_config";
import { General } from "./general";

export class Aptos {
  readonly config: AptosConfig;

  readonly account: Account;

  readonly general: General;

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

    this.account = new Account(this.config);
    this.general = new General(this.config);
  }
}
