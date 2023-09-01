import { Account } from "./account";
import { AptosConfig } from "./aptos_config";

export class Aptos {
  readonly config: AptosConfig;

  readonly account: Account;

  /**
   * This class is the main entry point into Aptos's
   * APIs and separates functionality into different namespaces.
   *
   * To use the SDK, create a new Aptos instance to get access
   * to all the sdk functionality.
   *
   * @example
   * ```
   * {
   * const aptos = new Aptos();
   * await aptos.account.getData("0x1")
   * }
   * ```
   *
   * @public
   */
  constructor(settings?: AptosConfig) {
    this.config = new AptosConfig(settings);
    this.account = new Account(this.config);
  }
}
