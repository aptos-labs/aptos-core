/* eslint-disable @typescript-eslint/naming-convention */

import { Serializer, Deserializer } from "../../bcs";
import { TransactionAuthenticator } from "../authenticator/transaction";
import { RawTransaction } from "./rawTransaction";

export class SignedTransaction {
  public readonly raw_txn: RawTransaction;

  public readonly authenticator: TransactionAuthenticator;

  /**
   * A SignedTransaction consists of a raw transaction and an authenticator. The authenticator
   * contains a client's public key and the signature of the raw transaction.
   *
   * @see {@link https://aptos.dev/integration/creating-a-signed-transaction | Creating a Signed Transaction}
   *
   * @param raw_txn
   * @param authenticator Contains a client's public key and the signature of the raw transaction.
   * Authenticator has 3 flavors: single signature, multi-signature and multi-agent.
   * @see {@link https://github.com/aptos-labs/aptos-core/blob/main/types/src/transaction/authenticator.rs} for details.
   */
  constructor(raw_txn: RawTransaction, authenticator: TransactionAuthenticator) {
    this.raw_txn = raw_txn;
    this.authenticator = authenticator;
  }

  serialize(serializer: Serializer): void {
    this.raw_txn.serialize(serializer);
    this.authenticator.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): SignedTransaction {
    const raw_txn = RawTransaction.deserialize(deserializer);
    const authenticator = TransactionAuthenticator.deserialize(deserializer);
    return new SignedTransaction(raw_txn, authenticator);
  }
}
