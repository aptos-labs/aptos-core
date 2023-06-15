import { Transaction } from "./transaction";
import { Account } from "./account";
import { AptosConfig } from "./aptos_config";
import { General } from "./general";

export class Aptos {
  readonly config: AptosConfig;
  readonly account: Account;
  readonly transaction: Transaction;
  readonly general: General;

  constructor(settings?: AptosConfig) {
    // can read from existing conf file or use the input settings param
    this.config = new AptosConfig(settings);

    this.account = new Account(this.config);
    this.transaction = new Transaction(this.config);
    this.general = new General(this.config);
  }
}
