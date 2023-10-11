import { Account } from "./account";
import { AptosConfig } from "./aptos_config";
import { General } from "./general";
import { Transaction } from "./transaction";
import { TransactionSubmission } from "./transaction_submission";

export class Aptos {
  readonly config: AptosConfig;

  readonly account: Account;

  readonly general: General;

  readonly transaction: Transaction;

  readonly transactionSubmission: TransactionSubmission;

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
   * await aptos.account.getInfo("0x1")
   * }
   * ```
   *
   * @public
   */
  constructor(settings?: AptosConfig) {
    this.config = new AptosConfig(settings);
    this.account = new Account(this.config);
    this.general = new General(this.config);
    this.transaction = new Transaction(this.config);
    this.transactionSubmission = new TransactionSubmission(this.config);
  }
}

export interface Aptos extends Account, General, Transaction, TransactionSubmission {}

/**
In TypeScript, we can’t inherit or extend from more than one class,
Mixins helps us to get around that by creating a partial classes 
that we can combine to form a single class that contains all the methods and properties from the partial classes.
{@link https://www.typescriptlang.org/docs/handbook/mixins.html#alternative-pattern}

Here, we combine any sub-class and the Aptos class.
*/
function applyMixin(targetClass: any, baseClass: any, baseClassProp: string) {
  // Mixin instance methods
  Object.getOwnPropertyNames(baseClass.prototype).forEach((propertyName) => {
    const propertyDescriptor = Object.getOwnPropertyDescriptor(baseClass.prototype, propertyName);
    if (!propertyDescriptor) return;
    // eslint-disable-next-line func-names
    propertyDescriptor.value = function (...args: any) {
      return (this as any)[baseClassProp][propertyName](...args);
    };
    Object.defineProperty(targetClass.prototype, propertyName, propertyDescriptor);
  });
}

applyMixin(Aptos, Account, "account");
applyMixin(Aptos, General, "general");
applyMixin(Aptos, Transaction, "transaction");
applyMixin(Aptos, TransactionSubmission, "transactionSubmission");
