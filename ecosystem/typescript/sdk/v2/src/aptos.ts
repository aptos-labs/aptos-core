import { Account } from "./api/account";
import { AptosConfig } from "./aptos_config";

export class Aptos {
  readonly config: AptosConfig;
  readonly account: Account;

  constructor(settings?: AptosConfig) {
    // can read from existing conf file or use the input settings param
    this.config = new AptosConfig(settings);

    this.account = new Account(this.config);
  }
}
