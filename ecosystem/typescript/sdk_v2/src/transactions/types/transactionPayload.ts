import { Serializer, Deserializer, Serializable } from "../../bcs";
import { AccountAddress } from "../../core";
import { Identifier } from "./identifier";
import { TransactionArgument } from "./transactionArguments";
import { ModuleId } from "./moduleId";

/**
 * Representation of the supported Transaction Payload
 * that can serialized and deserialized
 */
export abstract class TransactionPayload extends Serializable {
  /**
   * Serialize a Transaction Payload
   */
  abstract serialize(serializer: Serializer): void;

  /**
   * Deserialize a Transaction Payload
   */
  static deserialize(deserializer: Deserializer): TransactionPayload {
    const index = deserializer.deserializeUleb128AsU32();
    /**
     * index is represented in rust as an enum
     * {@link https://github.com/aptos-labs/aptos-core/blob/main/types/src/transaction/mod.rs#L478}
     */
    switch (index) {
      case 0:
        return TransactionPayloadScript.load(deserializer);
      // TODO: change to 1 once ModuleBundle has been removed from rust
      case 2:
        return TransactionPayloadEntryFunction.load(deserializer);
      case 3:
        return TransactionPayloadMultisig.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionPayload: ${index}`);
    }
  }
}

/**
 * Representation of a Transaction Payload Script that can serialized and deserialized
 */
export class TransactionPayloadScript extends TransactionPayload {
  public readonly script: Script;

  constructor(script: Script) {
    super();
    this.script = script;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    this.script.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadScript {
    const script = Script.deserialize(deserializer);
    return new TransactionPayloadScript(script);
  }
}

/**
 * Representation of a Transaction Payload Entry Function that can serialized and deserialized
 */
export class TransactionPayloadEntryFunction extends TransactionPayload {
  public readonly entryFunction: EntryFunction;

  constructor(entryFunction: EntryFunction) {
    super();
    this.entryFunction = entryFunction;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
    this.entryFunction.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadEntryFunction {
    const entryFunction = EntryFunction.deserialize(deserializer);
    return new TransactionPayloadEntryFunction(entryFunction);
  }
}

/**
 * Representation of a Transaction Payload Multisig that can serialized and deserialized
 */
export class TransactionPayloadMultisig extends TransactionPayload {
  public readonly multiSig: MultiSig;

  constructor(multiSig: MultiSig) {
    super();
    this.multiSig = multiSig;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(3);
    this.multiSig.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadMultisig {
    const multiSig = MultiSig.deserialize(deserializer);
    return new TransactionPayloadMultisig(multiSig);
  }
}

/**
 * Representation of a EntryFunction that can serialized and deserialized
 */
export class EntryFunction {
  public readonly module_name: ModuleId;

  public readonly function_name: Identifier;

  public readonly type_args: Array<TypeTag>;

  public readonly args: Array<Uint8Array>;

  /**
   * Contains the payload to run a function within a module.
   * @param module_name Fully qualified module name in format "account_address::module_name" e.g. "0x1::coin"
   * @param function_name The function name. e.g "transfer"
   * @param type_args Type arguments that move function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public entry fun transfer<CoinType>(from: &signer, to: address, amount: u64)
   * ```
   * @param args Arugments to the move function.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public entry fun transfer<CoinType>(from: &signer, to: address, amount: u64)
   * ```
   */
  constructor(module_name: ModuleId, function_name: Identifier, type_args: Array<TypeTag>, args: Array<Uint8Array>) {
    this.module_name = module_name;
    this.function_name = function_name;
    this.type_args = type_args;
    this.args = args;
  }

  serialize(serializer: Serializer): void {
    this.module_name.serialize(serializer);
    this.function_name.serialize(serializer);
    serializer.serializeVector<TypeTag>(this.ty_args);

    serializer.serializeU32AsUleb128(this.args.length);
    this.args.forEach((item: Uint8Array) => {
      serializer.serializeBytes(item);
    });
  }

  static deserialize(deserializer: Deserializer): EntryFunction {
    const module_name = ModuleId.deserialize(deserializer);
    const function_name = Identifier.deserialize(deserializer);
    const ty_args = deserializer.deserializeVector(TypeTag);

    const length = deserializer.deserializeUleb128AsU32();
    const list: Array<Uint8Array> = [];
    for (let i = 0; i < length; i += 1) {
      list.push(deserializer.deserializeBytes());
    }

    const args = list;
    return new EntryFunction(module_name, function_name, ty_args, args);
  }
}

/**
 * Representation of a Script that can serialized and deserialized
 */
export class Script {
  /**
   * The move module bytecode
   */
  public readonly bytecode: Uint8Array;

  /**
   * The type arguments that the bytecode function requires.
   */
  public readonly type_args: Array<TypeTag>;

  /**
   * The arugments that the bytecode function requires.
   */
  public readonly args: Array<TransactionArgument>;

  /**
   * Scripts contain the Move bytecodes payload that can be submitted to Aptos chain for execution.
   *
   * @param code The move module bytecode
   * @param type_args The type arguments that the bytecode function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args The arugments that the bytecode function requires.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   */
  constructor(bytecode: Uint8Array, type_args: Array<TypeTag>, args: Array<TransactionArgument>) {
    this.bytecode = bytecode;
    this.type_args = type_args;
    this.args = args;
  }

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.bytecode);
    serializer.serializeVector<TypeTag>(this.type_args);
    serializer.serializeVector<TransactionArgument>(this.args);
  }

  static deserialize(deserializer: Deserializer): Script {
    const bytecode = deserializer.deserializeBytes();
    const type_args = deserializer.deserializeVector(TypeTag);
    const args = deserializer.deserializeVector(TransactionArgument);
    return new Script(bytecode, type_args, args);
  }
}

/**
 * Representation of a MultiSig that can serialized and deserialized
 */
export class MultiSig {
  public readonly multisig_address: AccountAddress;

  public readonly transaction_payload?: MultiSigTransactionPayload;

  /**
   * Contains the payload to run a multisig account transaction.
   *
   * @param multisig_address The multisig account address the transaction will be executed as.
   *
   * @param transaction_payload The payload of the multisig transaction. This is optional when executing a multisig
   *  transaction whose payload is already stored on chain.
   */
  constructor(multisig_address: AccountAddress, transaction_payload?: MultiSigTransactionPayload) {
    this.multisig_address = multisig_address;
    this.transaction_payload = transaction_payload;
  }

  serialize(serializer: Serializer): void {
    this.multisig_address.serialize(serializer);
    // Options are encoded with an extra u8 field before the value - 0x0 is none and 0x1 is present.
    // We use serializeBool below to create this prefix value.
    if (this.transaction_payload === undefined) {
      serializer.serializeBool(false);
    } else {
      serializer.serializeBool(true);
      this.transaction_payload.serialize(serializer);
    }
  }

  static deserialize(deserializer: Deserializer): MultiSig {
    const multisig_address = AccountAddress.deserialize(deserializer);
    const payloadPresent = deserializer.deserializeBool();
    let transaction_payload;
    if (payloadPresent) {
      transaction_payload = MultiSigTransactionPayload.deserialize(deserializer);
    }
    return new MultiSig(multisig_address, transaction_payload);
  }
}

/**
 * Representation of a MultiSig Transaction Payload that can serialized and deserialized
 */
export class MultiSigTransactionPayload {
  public readonly transaction_payload: EntryFunction;

  /**
   * Contains the payload to run a multisig account transaction.
   *
   * @param transaction_payload The payload of the multisig transaction.
   * This can only be EntryFunction for now but,
   * Script might be supported in the future.
   */
  constructor(transaction_payload: EntryFunction) {
    this.transaction_payload = transaction_payload;
  }

  serialize(serializer: Serializer): void {
    /**
     * We can support multiple types of inner transaction payload in the future.
     * For now it's only EntryFunction but if we support more types,
     * we need to serialize with the right enum values here
     */
    serializer.serializeU32AsUleb128(0);
    this.transaction_payload.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): MultiSigTransactionPayload {
    // This is the enum value indicating which type of payload the multisig tx contains.
    deserializer.deserializeUleb128AsU32();
    return new MultiSigTransactionPayload(EntryFunction.deserialize(deserializer));
  }
}
